use anyhow::Result;
use rdev::{listen, Event, EventType};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputEvent {
    TriggerPressed,
    TriggerReleased,
}

pub struct InputMonitor {
    event_rx: Receiver<InputEvent>,
}

impl InputMonitor {
    pub fn new() -> Result<Self> {
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            if let Err(e) = run_listener(tx) {
                tracing::error!("input listener error: {}", e);
            }
        });

        Ok(Self { event_rx: rx })
    }

    pub fn try_recv(&self) -> Option<InputEvent> {
        self.event_rx.try_recv().ok()
    }
}

fn run_listener(tx: Sender<InputEvent>) -> Result<()> {
    // Track state to avoid duplicate events
    let mut is_pressed = false;

    listen(move |event: Event| {
        match event.event_type {
            // Mouse button 4 (forward/gesture button on Logitech mice)
            // On MX Master, this maps to Button::Unknown(3) or similar
            // We also support F18 as a fallback hotkey (can be mapped in Logi Options+)
            EventType::ButtonPress(button) => {
                // Log all button presses to help identify the correct button code
                tracing::debug!("button press detected: {:?}", button);

                let is_trigger = matches!(
                    button,
                    rdev::Button::Unknown(3)  // Often the gesture button
                        | rdev::Button::Unknown(4)
                        | rdev::Button::Unknown(5)
                        | rdev::Button::Unknown(6)
                        | rdev::Button::Unknown(8)  // Some mice report as 8
                );

                if is_trigger && !is_pressed {
                    is_pressed = true;
                    tracing::info!("trigger button pressed: {:?}", button);
                    let _ = tx.send(InputEvent::TriggerPressed);
                }
            }
            EventType::ButtonRelease(button) => {
                let is_trigger = matches!(
                    button,
                    rdev::Button::Unknown(3)
                        | rdev::Button::Unknown(4)
                        | rdev::Button::Unknown(5)
                        | rdev::Button::Unknown(6)
                        | rdev::Button::Unknown(8)
                );

                if is_trigger && is_pressed {
                    is_pressed = false;
                    tracing::info!("trigger button released: {:?}", button);
                    let _ = tx.send(InputEvent::TriggerReleased);
                }
            }
            // Support Right Option key and Logitech gesture button
            // Gesture button sends KEY PRESS Unknown(65535) on press, KEY PRESS KeyA on release
            EventType::KeyPress(key) => {
                if matches!(key, rdev::Key::AltGr | rdev::Key::Unknown(65535)) && !is_pressed {
                    is_pressed = true;
                    tracing::info!("trigger pressed: {:?}", key);
                    let _ = tx.send(InputEvent::TriggerPressed);
                } else if matches!(key, rdev::Key::KeyA) && is_pressed {
                    // Logitech gesture button release comes as KEY PRESS KeyA
                    is_pressed = false;
                    tracing::info!("trigger released (gesture button): {:?}", key);
                    let _ = tx.send(InputEvent::TriggerReleased);
                }
            }
            EventType::KeyRelease(key) => {
                // Normal key release for Right Option
                if matches!(key, rdev::Key::AltGr) && is_pressed {
                    is_pressed = false;
                    tracing::info!("trigger released: {:?}", key);
                    let _ = tx.send(InputEvent::TriggerReleased);
                }
            }
            _ => {}
        }
    })
    .map_err(|e| anyhow::anyhow!("failed to start input listener: {:?}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_event_equality() {
        assert_eq!(InputEvent::TriggerPressed, InputEvent::TriggerPressed);
        assert_ne!(InputEvent::TriggerPressed, InputEvent::TriggerReleased);
    }
}
