use std::fs;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use evdev::{Device, EventType};

// Maximum bounds for Gaomon WH851 coordinates
pub const MAX_X: f32 = 40640.0;
pub const MAX_Y: f32 = 25400.0;
pub const MAX_PRESSURE: f32 = 16383.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TabletButton {
    Stylus1,
    Stylus2,
    Express1,
    Express2,
    Express3,
    Express4,
    Express5,
    Express6,
    Express7,
    Express8,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TabletEvent {
    PenMotion {
        x: f32,        // Normalized 0.0 to 1.0
        y: f32,        // Normalized 0.0 to 1.0
        pressure: f32, // Normalized 0.0 to 1.0
        tilt_x: i32,
        tilt_y: i32,
    },
    PenTouch {
        touching: bool,
    },
    Button {
        button: TabletButton,
        pressed: bool,
    },
    Proximity {
        in_range: bool,
    },
}

pub struct TabletListener {
    _threads: Vec<thread::JoinHandle<()>>,
    receiver: Receiver<TabletEvent>,
}

impl TabletListener {
    /// Auto-detects Gaomon WH851 Pen and Keyboard input streams and starts background parsing threads.
    pub fn start() -> Result<Self, std::io::Error> {
        let (pen_path, kbd_path) = Self::find_devices();
        
        if pen_path.is_none() && kbd_path.is_none() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No Gaomon WH851 devices found.",
            ));
        }

        let (sender, receiver) = channel();
        let mut threads = Vec::new();

        // 1. Spawning Pen Reader Thread
        if let Some(path) = pen_path {
            let tx = sender.clone();
            threads.push(thread::spawn(move || {
                if let Err(e) = Self::run_pen_listener(&path, tx) {
                    eprintln!("Error in pen listener thread: {:?}", e);
                }
            }));
        }

        // 2. Spawning Keyboard/Express Button Reader Thread
        if let Some(path) = kbd_path {
            let tx = sender;
            threads.push(thread::spawn(move || {
                if let Err(e) = Self::run_keyboard_listener(&path, tx) {
                    eprintln!("Error in keyboard listener thread: {:?}", e);
                }
            }));
        }

        Ok(Self {
            _threads: threads,
            receiver,
        })
    }

    /// Try to receive a pending event without blocking.
    pub fn try_recv(&self) -> Option<TabletEvent> {
        self.receiver.try_recv().ok()
    }

    /// Blocking read for next event.
    pub fn recv(&self) -> Option<TabletEvent> {
        self.receiver.recv().ok()
    }

    /// Iterates over evdev devices dynamically to find the correct WH851 file paths.
    fn find_devices() -> (Option<String>, Option<String>) {
        let mut pen_path = None;
        let mut kbd_path = None;
        if let Ok(entries) = fs::read_dir("/dev/input") {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
                    if filename.starts_with("event") {
                        if let Ok(device) = Device::open(&path) {
                            if let Some(name) = device.name() {
                                if name.contains("WH851") && name.contains("Pen") {
                                    pen_path = Some(path.to_string_lossy().into_owned());
                                } else if name.contains("WH851") && name.contains("Keyboard") {
                                    kbd_path = Some(path.to_string_lossy().into_owned());
                                }
                            }
                        }
                    }
                }
            }
        }
        (pen_path, kbd_path)
    }

    fn run_pen_listener(path: &str, sender: Sender<TabletEvent>) -> Result<(), std::io::Error> {
        let mut device = Device::open(path)?;
        
        // Cache variables to track state changes and group ABS updates
        let mut last_x: f32 = 0.0;
        let mut last_y: f32 = 0.0;
        let mut last_pressure: f32 = 0.0;
        let mut last_tilt_x: i32 = 0;
        let mut last_tilt_y: i32 = 0;
        let mut abs_changed = false;

        // Per-SYN-frame batch accumulators
        // We collect all events in a single hardware frame (between SYN_REPORTs)
        // and then process them together. This lets us detect when BTN_STYLUS
        // release arrives in the same frame as BTN_TOUCH press — which is the
        // Gaomon firmware forcibly releasing the barrel button on pen contact.
        let mut frame_stylus1_press = false;
        let mut frame_stylus1_release = false;
        let mut frame_stylus2_press = false;
        let mut frame_stylus2_release = false;
        let mut frame_touch_press = false;
        let mut frame_touch_release = false;
        let mut frame_proximity: Option<bool> = None;

        // Persistent physical button state (survives across frames)
        let mut stylus1_held = false;
        let mut stylus2_held = false;

        loop {
            for event in device.fetch_events()? {
                match event.event_type() {
                    EventType::ABSOLUTE => {
                        let val = event.value();
                        match event.code() {
                            0 => { // ABS_X
                                last_x = val as f32 / MAX_X;
                                abs_changed = true;
                            }
                            1 => { // ABS_Y
                                last_y = val as f32 / MAX_Y;
                                abs_changed = true;
                            }
                            24 => { // ABS_PRESSURE
                                last_pressure = val as f32 / MAX_PRESSURE;
                                abs_changed = true;
                            }
                            26 => { // ABS_TILT_X
                                last_tilt_x = val;
                                abs_changed = true;
                            }
                            27 => { // ABS_TILT_Y
                                last_tilt_y = val;
                                abs_changed = true;
                            }
                            _ => {}
                        }
                    }
                    EventType::KEY => {
                        match event.code() {
                            330 => { // BTN_TOUCH
                                if event.value() != 0 {
                                    frame_touch_press = true;
                                } else {
                                    frame_touch_release = true;
                                }
                            }
                            320 => { // BTN_TOOL_PEN
                                frame_proximity = Some(event.value() != 0);
                            }
                            _ => {}
                        }
                    }
                    EventType::SYNCHRONIZATION => {
                        // ── Process the completed SYN frame ──

                        // 1. Proximity
                        if let Some(in_range) = frame_proximity.take() {
                            let _ = sender.send(TabletEvent::Proximity { in_range });
                            if !in_range {
                                // Pen left range — force-release held buttons
                                if stylus1_held {
                                    stylus1_held = false;
                                    let _ = sender.send(TabletEvent::Button {
                                        button: TabletButton::Stylus1, pressed: false,
                                    });
                                }
                                if stylus2_held {
                                    stylus2_held = false;
                                    let _ = sender.send(TabletEvent::Button {
                                        button: TabletButton::Stylus2, pressed: false,
                                    });
                                }
                            }
                        }

                        // 2. Stylus1 barrel button
                        if frame_stylus1_press {
                            stylus1_held = true;
                            let _ = sender.send(TabletEvent::Button {
                                button: TabletButton::Stylus1, pressed: true,
                            });
                        }
                        if frame_stylus1_release {
                            if frame_touch_press {
                                // FIRMWARE SUPPRESSION DETECTED:
                                // BTN_STYLUS released in the same frame as BTN_TOUCH pressed.
                                // The user is still physically holding the button — ignore this release.
                            } else {
                                // Genuine release (user lifted finger off barrel button)
                                stylus1_held = false;
                                let _ = sender.send(TabletEvent::Button {
                                    button: TabletButton::Stylus1, pressed: false,
                                });
                            }
                        }

                        // 3. Stylus2 barrel button (same suppression logic)
                        if frame_stylus2_press {
                            stylus2_held = true;
                            let _ = sender.send(TabletEvent::Button {
                                button: TabletButton::Stylus2, pressed: true,
                            });
                        }
                        if frame_stylus2_release {
                            if frame_touch_press {
                                // Firmware suppression — ignore
                            } else {
                                stylus2_held = false;
                                let _ = sender.send(TabletEvent::Button {
                                    button: TabletButton::Stylus2, pressed: false,
                                });
                            }
                        }

                        // 4. Touch events
                        if frame_touch_press {
                            let _ = sender.send(TabletEvent::PenTouch { touching: true });
                        }
                        if frame_touch_release {
                            let _ = sender.send(TabletEvent::PenTouch { touching: false });
                            // When pen lifts, if a button was firmware-suppressed and the
                            // user has since released it physically, we need to emit the
                            // deferred release. We check: if the kernel thinks stylus is
                            // NOT held (no BTN_STYLUS down), but our state says held,
                            // then emit the release now.
                            // (This is handled naturally: next frame without BTN_STYLUS
                            //  events won't have frame_stylus1_release set, so no spurious
                            //  releases. If user truly releases the button later, a clean
                            //  BTN_STYLUS 0 event arrives in a frame WITHOUT BTN_TOUCH
                            //  press, and passes through normally.)
                        }

                        // 5. Accumulated absolute axis motion
                        if abs_changed {
                            let _ = sender.send(TabletEvent::PenMotion {
                                x: last_x.clamp(0.0, 1.0),
                                y: last_y.clamp(0.0, 1.0),
                                pressure: last_pressure.clamp(0.0, 1.0),
                                tilt_x: last_tilt_x,
                                tilt_y: last_tilt_y,
                            });
                            abs_changed = false;
                        }

                        // Reset frame accumulators
                        frame_stylus1_press = false;
                        frame_stylus1_release = false;
                        frame_stylus2_press = false;
                        frame_stylus2_release = false;
                        frame_touch_press = false;
                        frame_touch_release = false;
                    }
                    _ => {}
                }
            }
        }
    }

    fn run_keyboard_listener(path: &str, sender: Sender<TabletEvent>) -> Result<(), std::io::Error> {
        let mut device = Device::open(path)?;

        // Track active keys to parse chord combinations for Buttons 7 & 8
        // Remapped key mapping reference:
        // F13 (183) = KEY_B / Stylus1 & Express1
        // F14 (184) = KEY_E / Stylus2 & Express2
        // F15 (185) = Express3
        // F16 (186) = Express4
        // F17 (187) = Express5
        // F18 (188) = Express6
        // F19 (189) = Ctrl remap modifier
        // F20 (190) = Alt remap modifier
        // F21 (191) = Z remap modifier
        
        let mut f13_down = false;
        let mut f14_down = false;
        let mut f15_down = false;
        let mut f16_down = false;
        let mut f17_down = false;
        let mut f18_down = false;
        let mut f19_down = false;
        let mut f20_down = false;
        let mut f21_down = false;

        loop {
            for event in device.fetch_events()? {
                if event.event_type() == EventType::KEY {
                    let key = event.code();
                    let is_down = event.value() == 1 || event.value() == 2; // 1 = down, 2 = repeat
                    let is_initial_press = event.value() == 1; // Only true on first press, NOT repeat
                    let is_release = event.value() == 0;

                    // Update key states (including repeats for chord detection)
                    match key {
                        183 => f13_down = is_down,
                        184 => f14_down = is_down,
                        185 => f15_down = is_down,
                        186 => f16_down = is_down,
                        187 => f17_down = is_down,
                        188 => f18_down = is_down,
                        189 => f19_down = is_down,
                        190 => f20_down = is_down,
                        191 => f21_down = is_down,
                        _ => {}
                    }

                    // Only emit button events on INITIAL press (not repeat) to prevent
                    // toggle flickering when holding a button down.
                    if is_initial_press {
                        // Button 8 (F19 + F20 + F21)
                        if f19_down && f20_down && f21_down {
                            let _ = sender.send(TabletEvent::Button { button: TabletButton::Express8, pressed: true });
                        }
                        // Button 7 (F19 + F14)
                        else if f19_down && f14_down {
                            let _ = sender.send(TabletEvent::Button { button: TabletButton::Express7, pressed: true });
                        }
                        // Single express keys (one event per key, no duplicates)
                        else if key == 183 {
                            let _ = sender.send(TabletEvent::Button { button: TabletButton::Express1, pressed: true });
                        }
                        else if key == 184 && !f19_down {
                            let _ = sender.send(TabletEvent::Button { button: TabletButton::Express2, pressed: true });
                        }
                        else if key == 185 {
                            let _ = sender.send(TabletEvent::Button { button: TabletButton::Express3, pressed: true });
                        }
                        else if key == 186 {
                            let _ = sender.send(TabletEvent::Button { button: TabletButton::Express4, pressed: true });
                        }
                        else if key == 187 {
                            let _ = sender.send(TabletEvent::Button { button: TabletButton::Express5, pressed: true });
                        }
                        else if key == 188 {
                            let _ = sender.send(TabletEvent::Button { button: TabletButton::Express6, pressed: true });
                        }
                    } else if is_release {
                        match key {
                            // F13 and F14 releases are NEVER forwarded.
                            // This prevents firmware button suppression from
                            // interfering with toggle-based mode switching.
                            183 => {}
                            184 => {}
                            185 => {
                                let _ = sender.send(TabletEvent::Button { button: TabletButton::Express3, pressed: false });
                            }
                            186 => {
                                let _ = sender.send(TabletEvent::Button { button: TabletButton::Express4, pressed: false });
                            }
                            187 => {
                                let _ = sender.send(TabletEvent::Button { button: TabletButton::Express5, pressed: false });
                            }
                            188 => {
                                let _ = sender.send(TabletEvent::Button { button: TabletButton::Express6, pressed: false });
                            }
                            191 => {
                                let _ = sender.send(TabletEvent::Button { button: TabletButton::Express8, pressed: false });
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }
}
