use std::time::Duration;
use tablet_input::TabletListener;

fn main() {
    println!("Starting Gaomon WH851 Hardware monitor...");
    println!("Listening for pen proximity, motion, pressure, and remapped buttons (F13-F21).");
    
    match TabletListener::start() {
        Ok(listener) => {
            println!("Driver started successfully! Try moving the pen or pressing buttons...");
            loop {
                if let Some(event) = listener.recv() {
                    println!("Event received: {:?}", event);
                }
                std::thread::sleep(Duration::from_millis(5));
            }
        }
        Err(e) => {
            eprintln!("Failed to start tablet listener: {:?}", e);
            eprintln!("Please ensure the tablet is paired via Bluetooth and you have permission to read /dev/input/event* (are you in the 'input' group?).");
        }
    }
}
