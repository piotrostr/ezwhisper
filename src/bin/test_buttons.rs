use rdev::{listen, Event, EventType};

fn main() {
    println!("Press any mouse button or key. Ctrl-C to quit.");
    println!("Looking for your gesture button code...\n");

    listen(|event: Event| {
        match event.event_type {
            EventType::ButtonPress(b) => println!("BUTTON PRESS: {:?}", b),
            EventType::ButtonRelease(b) => println!("BUTTON RELEASE: {:?}", b),
            EventType::KeyPress(k) => println!("KEY PRESS: {:?}", k),
            EventType::KeyRelease(k) => println!("KEY RELEASE: {:?}", k),
            _ => {}
        }
    }).unwrap();
}
