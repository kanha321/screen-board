#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use egui::{Color32, Frame, Pos2, Rgba, Stroke as EguiStroke, Vec2};
use tablet_input::{TabletButton, TabletEvent, TabletListener};

const DEFAULT_BRUSH_SIZE: f32 = 4.0;
const ERASER_RADIUS: f32 = 25.0;

struct Point {
    pos: Pos2,
    width: f32,
}

struct Stroke {
    points: Vec<Point>,
    color: Color32,
}

struct ScreenBoardApp {
    listener: Option<TabletListener>,
    strokes: Vec<Stroke>,
    current_stroke: Option<Stroke>,
    
    // Brush settings
    selected_color: Color32,
    brush_size: f32,
    
    // Modes
    eraser_mode: bool,
    passthrough: bool,
    
    // Live Pen/Cursor Telemetry
    pen_pos: Pos2,
    pen_pressure: f32,
    pen_in_range: bool,
}

impl ScreenBoardApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Configure theme to dark glassmorphism
        let mut visuals = egui::Visuals::dark();
        visuals.window_rounding = 12.0.into();
        visuals.widgets.noninteractive.bg_fill = Color32::from_black_alpha(150);
        visuals.widgets.noninteractive.fg_stroke = EguiStroke::new(1.0, Color32::from_gray(100));
        cc.egui_ctx.set_visuals(visuals);

        // Start tablet listener
        let listener = match TabletListener::start() {
            Ok(l) => {
                println!("Successfully started tablet listener.");
                Some(l)
            }
            Err(e) => {
                eprintln!("Failed to start tablet listener: {:?}", e);
                None
            }
        };

        Self {
            listener,
            strokes: Vec::new(),
            current_stroke: None,
            selected_color: Color32::from_rgb(0, 240, 255), // Neon Cyan default
            brush_size: DEFAULT_BRUSH_SIZE,
            eraser_mode: false,
            passthrough: false,
            pen_pos: Pos2::ZERO,
            pen_pressure: 0.0,
            pen_in_range: false,
        }
    }

    fn undo(&mut self) {
        self.strokes.pop();
    }

    fn clear_canvas(&mut self) {
        self.strokes.clear();
        self.current_stroke = None;
    }

    fn process_tablet_events(&mut self, ctx: &egui::Context) {
        let Some(listener) = &self.listener else { return };
        
        let mut event_received = false;
        let mut events = Vec::new();
        while let Some(event) = listener.try_recv() {
            events.push(event);
        }

        for event in events {
            event_received = true;
            match event {
                TabletEvent::Proximity { in_range } => {
                    self.pen_in_range = in_range;
                    if !in_range {
                        // Pen left: finalize current stroke
                        if let Some(stroke) = self.current_stroke.take() {
                            self.strokes.push(stroke);
                        }
                    }
                }
                TabletEvent::PenMotion { x, y, pressure, .. } => {
                    let screen_rect = ctx.screen_rect();
                    let px = x * screen_rect.width() + screen_rect.min.x;
                    let py = y * screen_rect.height() + screen_rect.min.y;
                    self.pen_pos = egui::pos2(px, py);
                    self.pen_pressure = pressure;

                    // Handle active drawing/erasing if pen is touching screen
                    if self.pen_pressure > 0.0 {
                        let active_width = self.brush_size * (0.2 + 0.8 * self.pen_pressure);

                        if self.eraser_mode {
                            self.erase_strokes_at(self.pen_pos);
                        } else {
                            if let Some(stroke) = &mut self.current_stroke {
                                if stroke.points.is_empty() || stroke.points.last().unwrap().pos.distance_sq(self.pen_pos) > 1.0 {
                                    stroke.points.push(Point {
                                        pos: self.pen_pos,
                                        width: active_width,
                                    });
                                }
                            } else {
                                self.current_stroke = Some(Stroke {
                                    points: vec![Point {
                                        pos: self.pen_pos,
                                        width: active_width,
                                    }],
                                    color: self.selected_color,
                                });
                            }
                        }
                    } else {
                        // Lifted: finalize current stroke
                        if let Some(stroke) = self.current_stroke.take() {
                            self.strokes.push(stroke);
                        }
                    }
                }
                TabletEvent::PenTouch { touching } => {
                    if !touching {
                        if let Some(stroke) = self.current_stroke.take() {
                            self.strokes.push(stroke);
                        }
                    }
                }
                TabletEvent::Button { button, pressed } => {
                    match button {
                        TabletButton::Express1 => {
                            // F13: toggle eraser mode
                            if pressed {
                                self.eraser_mode = !self.eraser_mode;
                            }
                        }
                        TabletButton::Express2 => {
                            // F14: undo
                            if pressed {
                                self.undo();
                            }
                        }
                        TabletButton::Express3 => {
                            if pressed {
                                self.clear_canvas();
                            }
                        }
                        TabletButton::Express4 => {
                            if pressed {
                                self.passthrough = !self.passthrough;
                                ctx.send_viewport_cmd(egui::ViewportCommand::MousePassthrough(self.passthrough));
                            }
                        }
                        TabletButton::Express5 => {
                            if pressed {
                                self.cycle_colors();
                            }
                        }
                        TabletButton::Express6 => {
                            if pressed {
                                self.cycle_brush_sizes();
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        if event_received {
            ctx.request_repaint();
        }
    }

    /// Stroke-erasing algorithm checking distance to all active line segments
    fn erase_strokes_at(&mut self, eraser_pos: Pos2) {
        let r = ERASER_RADIUS;
        let mut to_remove = Vec::new();

        for (idx, stroke) in self.strokes.iter().enumerate() {
            if stroke.points.len() < 2 {
                continue;
            }
            // Check each line segment in the stroke
            for window in stroke.points.windows(2) {
                let p1 = window[0].pos;
                let p2 = window[1].pos;

                if line_segment_distance(eraser_pos, p1, p2) < r {
                    to_remove.push(idx);
                    break; // No need to check other segments of this stroke
                }
            }
        }

        // Delete from back to front to preserve indices
        to_remove.sort_unstable();
        for &idx in to_remove.iter().rev() {
            self.strokes.remove(idx);
        }
    }

    fn cycle_colors(&mut self) {
        let colors = [
            Color32::from_rgb(0, 240, 255),  // Neon Cyan
            Color32::from_rgb(255, 0, 127),  // Neon Pink
            Color32::from_rgb(0, 255, 127),  // Neon Green
            Color32::from_rgb(255, 230, 0),  // Yellow
            Color32::WHITE,
        ];
        if let Some(pos) = colors.iter().position(|&c| c == self.selected_color) {
            self.selected_color = colors[(pos + 1) % colors.len()];
        } else {
            self.selected_color = colors[0];
        }
    }

    fn cycle_brush_sizes(&mut self) {
        let sizes = [4.0, 8.0, 12.0, 18.0, 25.0];
        if let Some(pos) = sizes.iter().position(|&s| s == self.brush_size) {
            self.brush_size = sizes[(pos + 1) % sizes.len()];
        } else {
            self.brush_size = sizes[0];
        }
    }
}

/// Helper function to compute the shortest distance from point `e` to line segment `p1` -> `p2`
fn line_segment_distance(e: Pos2, p1: Pos2, p2: Pos2) -> f32 {
    let segment_len_sq = p1.distance_sq(p2);
    if segment_len_sq == 0.0 {
        return e.distance(p1);
    }
    
    // Parameterized line projection t
    let t = ((e.x - p1.x) * (p2.x - p1.x) + (e.y - p1.y) * (p2.y - p1.y)) / segment_len_sq;
    let t_clamped = t.clamp(0.0, 1.0);
    
    let closest_point = Pos2::new(
        p1.x + t_clamped * (p2.x - p1.x),
        p1.y + t_clamped * (p2.y - p1.y),
    );
    
    e.distance(closest_point)
}

impl eframe::App for ScreenBoardApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        Rgba::TRANSPARENT.to_array()
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 1. Process tablet-specific events (motion, pressure, buttons)
        self.process_tablet_events(ctx);

        // 2. Hide OS system cursor inside window so we can replace it with our own cursor
        ctx.set_cursor_icon(egui::CursorIcon::None);

        // 3. Mouse fallback drawing (when stylus is not active)
        if !self.pen_in_range {
            let pointer_pos = ctx.input(|i| i.pointer.hover_pos());
            let pointer_down = ctx.input(|i| i.pointer.any_down());

            if let Some(pos) = pointer_pos {
                self.pen_pos = pos;

                if pointer_down {
                    let active_width = self.brush_size; // Mouse has constant pressure (1.0)

                    if self.eraser_mode {
                        self.erase_strokes_at(pos);
                    } else {
                        if let Some(stroke) = &mut self.current_stroke {
                            if stroke.points.is_empty() || stroke.points.last().unwrap().pos.distance_sq(pos) > 1.0 {
                                stroke.points.push(Point { pos, width: active_width });
                                ctx.request_repaint();
                            }
                        } else {
                            self.current_stroke = Some(Stroke {
                                points: vec![Point { pos, width: active_width }],
                                color: self.selected_color,
                            });
                            ctx.request_repaint();
                        }
                    }
                } else {
                    if let Some(stroke) = self.current_stroke.take() {
                        self.strokes.push(stroke);
                        ctx.request_repaint();
                    }
                }
            }
        }

        // Setup a full-screen, transparent panel
        let frame = Frame::none().fill(Color32::TRANSPARENT);
        egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
            let painter = ui.painter();

            // 1. Draw all completed strokes
            for stroke in &self.strokes {
                if stroke.points.len() < 2 {
                    continue;
                }
                for window in stroke.points.windows(2) {
                    let p1 = window[0].pos;
                    let p2 = window[1].pos;
                    let avg_width = (window[0].width + window[1].width) / 2.0;
                    painter.line_segment([p1, p2], EguiStroke::new(avg_width, stroke.color));
                }
            }

            // 2. Draw current in-progress stroke
            if let Some(stroke) = &self.current_stroke {
                if stroke.points.len() >= 2 {
                    for window in stroke.points.windows(2) {
                        let p1 = window[0].pos;
                        let p2 = window[1].pos;
                        let avg_width = (window[0].width + window[1].width) / 2.0;
                        painter.line_segment([p1, p2], EguiStroke::new(avg_width, stroke.color));
                    }
                }
            }

            // 3. Render visual indicator for eraser or pen brush when hovering
            // We show the hover preview if the stylus is in range, or if the mouse is inside the window
            let show_cursor = self.pen_in_range || (!self.pen_in_range && ctx.input(|i| i.pointer.hover_pos()).is_some());
            
            if show_cursor && !self.passthrough {
                if self.eraser_mode {
                    painter.circle_stroke(
                        self.pen_pos,
                        ERASER_RADIUS,
                        EguiStroke::new(1.5, Color32::from_rgb(255, 0, 100)),
                    );
                    painter.circle_filled(self.pen_pos, 2.0, Color32::from_rgb(255, 0, 100));
                } else {
                    let pressure = if self.listener.is_some() && self.pen_pressure > 0.0 {
                        self.pen_pressure
                    } else {
                        1.0
                    };
                    let cursor_radius = self.brush_size * (0.2 + 0.8 * pressure) / 2.0;
                    painter.circle_stroke(
                        self.pen_pos,
                        cursor_radius.max(3.0),
                        EguiStroke::new(1.5, self.selected_color),
                    );
                    painter.circle_filled(self.pen_pos, 2.0, self.selected_color);
                }
            }

            // 4. Floating Control Panel (Only interactable if NOT in passthrough mode)
            if !self.passthrough {
                egui::Window::new("Screen Board Controls")
                    .anchor(egui::Align2::LEFT_TOP, Vec2::new(20.0, 20.0))
                    .resizable(false)
                    .collapsible(true)
                    .frame(
                        Frame::window(&ctx.style())
                            .fill(Color32::from_black_alpha(200))
                            .stroke(EguiStroke::new(1.0, Color32::from_gray(60))),
                    )
                    .show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            // Mode indicators
                            ui.label("Mode:");
                            if self.eraser_mode {
                                ui.colored_label(Color32::from_rgb(255, 0, 100), "Eraser");
                            } else {
                                ui.colored_label(self.selected_color, "Pen");
                            }
                        });

                        ui.separator();

                        // Color selection buttons
                        ui.label("Colors:");
                        ui.horizontal(|ui| {
                            let colors = [
                                (Color32::from_rgb(0, 240, 255), "Cyan"),
                                (Color32::from_rgb(255, 0, 127), "Pink"),
                                (Color32::from_rgb(0, 255, 127), "Green"),
                                (Color32::from_rgb(255, 230, 0), "Yellow"),
                                (Color32::WHITE, "White"),
                            ];
                            for (color, label) in colors {
                                let stroke = if self.selected_color == color {
                                    EguiStroke::new(2.0, Color32::WHITE)
                                } else {
                                    EguiStroke::NONE
                                };
                                if ui.add(
                                    egui::Button::new("     ")
                                        .stroke(stroke)
                                        .fill(color)
                                )
                                .on_hover_text(label)
                                .clicked() 
                                {
                                    self.selected_color = color;
                                    self.eraser_mode = false;
                                }
                            }
                        });

                        ui.add_space(4.0);

                        // Brush size slider
                        ui.horizontal(|ui| {
                            ui.label("Size:");
                            ui.add(egui::Slider::new(&mut self.brush_size, 1.0..=30.0).text("px"));
                        });

                        ui.separator();

                        ui.horizontal(|ui| {
                            if ui.button("Undo").clicked() {
                                self.undo();
                            }
                            if ui.button("Clear").clicked() {
                                self.clear_canvas();
                            }
                        });

                        ui.add_space(4.0);

                        if ui.button("Enable Desktop Passthrough").clicked() {
                            self.passthrough = true;
                            ctx.send_viewport_cmd(egui::ViewportCommand::MousePassthrough(true));
                        }
                    });
            } else {
                // If passthrough is enabled, display a small non-interactive reminder text in the corner
                egui::Area::new(egui::Id::new("passthrough_indicator"))
                    .anchor(egui::Align2::LEFT_TOP, Vec2::new(20.0, 20.0))
                    .show(ctx, |ui| {
                        Frame::dark_canvas(&ctx.style())
                            .fill(Color32::from_black_alpha(180))
                            .stroke(EguiStroke::new(1.0, Color32::from_rgb(0, 255, 100)))
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new("Desktop Mode Active (Click-Through)\nPress Express Key 4 to resume drawing.")
                                        .color(Color32::from_rgb(0, 255, 100))
                                        .strong()
                                );
                            });
                    });
            }
        });

        // Force continuous VSync rendering to prevent Wayland from sleeping the frame rendering loop
        ctx.request_repaint();
    }
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Screen Board Overlay")
            .with_transparent(true)
            .with_decorations(false)
            .with_fullscreen(true)
            .with_always_on_top()
            .with_mouse_passthrough(false),
        ..Default::default()
    };

    eframe::run_native(
        "screen_board",
        options,
        Box::new(|cc| Ok(Box::new(ScreenBoardApp::new(cc)))),
    )
}
