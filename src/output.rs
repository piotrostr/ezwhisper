use anyhow::Result;
use std::process::Command;

#[cfg(target_os = "macos")]
use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation, CGKeyCode};
#[cfg(target_os = "macos")]
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

pub struct TextInserter {
    auto_enter: bool,
}

impl TextInserter {
    pub fn new(auto_enter: bool) -> Self {
        Self { auto_enter }
    }

    #[cfg(target_os = "macos")]
    pub fn insert(&self, text: &str) -> Result<()> {
        if text.is_empty() {
            return Ok(());
        }

        tracing::info!("inserting {} chars via clipboard", text.len());

        // Copy text to clipboard using pbcopy
        let mut child = Command::new("pbcopy")
            .stdin(std::process::Stdio::piped())
            .spawn()?;

        if let Some(stdin) = child.stdin.as_mut() {
            use std::io::Write;
            stdin.write_all(text.as_bytes())?;
        }
        child.wait()?;

        // Small delay to ensure clipboard is ready
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Simulate Cmd+V to paste
        self.paste()?;

        // Optionally press Enter
        if self.auto_enter {
            // Wait for Cmd key to be fully released from paste
            std::thread::sleep(std::time::Duration::from_millis(150));
            self.press_enter()?;
        }

        Ok(())
    }

    #[cfg(target_os = "macos")]
    fn press_enter(&self) -> Result<()> {
        let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
            .map_err(|_| anyhow::anyhow!("failed to create event source"))?;

        // Return/Enter key is keycode 36
        let enter_keycode: CGKeyCode = 36;

        let key_down = CGEvent::new_keyboard_event(source.clone(), enter_keycode, true)
            .map_err(|_| anyhow::anyhow!("failed to create key down event"))?;

        let key_up = CGEvent::new_keyboard_event(source.clone(), enter_keycode, false)
            .map_err(|_| anyhow::anyhow!("failed to create key up event"))?;

        // Explicitly clear all modifier flags
        key_down.set_flags(CGEventFlags::CGEventFlagNull);
        key_up.set_flags(CGEventFlags::CGEventFlagNull);

        key_down.post(CGEventTapLocation::HID);
        key_up.post(CGEventTapLocation::HID);

        Ok(())
    }

    #[cfg(target_os = "macos")]
    fn paste(&self) -> Result<()> {
        let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
            .map_err(|_| anyhow::anyhow!("failed to create event source"))?;

        // V key is keycode 9
        let v_keycode: CGKeyCode = 9;

        let key_down = CGEvent::new_keyboard_event(source.clone(), v_keycode, true)
            .map_err(|_| anyhow::anyhow!("failed to create key down event"))?;

        let key_up = CGEvent::new_keyboard_event(source.clone(), v_keycode, false)
            .map_err(|_| anyhow::anyhow!("failed to create key up event"))?;

        // Set Cmd flag
        key_down.set_flags(CGEventFlags::CGEventFlagCommand);
        key_up.set_flags(CGEventFlags::CGEventFlagCommand);

        key_down.post(CGEventTapLocation::HID);
        key_up.post(CGEventTapLocation::HID);

        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    pub fn insert(&self, text: &str) -> Result<()> {
        tracing::warn!("text insertion not implemented for this platform");
        tracing::info!("would insert: {}", text);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inserter_creation() {
        let _inserter = TextInserter::new();
    }
}
