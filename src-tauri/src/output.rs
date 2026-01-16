use anyhow::Result;
use arboard::Clipboard;

pub struct TextInserter {
    auto_enter: bool,
}

impl TextInserter {
    pub fn new(auto_enter: bool) -> Self {
        Self { auto_enter }
    }

    pub fn insert(&self, text: &str) -> Result<()> {
        if text.is_empty() {
            return Ok(());
        }

        tracing::info!("inserting {} chars via clipboard", text.len());

        // Copy text to clipboard
        let mut clipboard = Clipboard::new()?;
        clipboard.set_text(text)?;

        // Small delay to ensure clipboard is ready
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Simulate Cmd+V using CGEvent (thread-safe, unlike enigo)
        simulate_paste()?;

        // Optionally press Enter
        if self.auto_enter {
            std::thread::sleep(std::time::Duration::from_millis(150));
            simulate_return()?;
        }

        Ok(())
    }
}

#[cfg(target_os = "macos")]
fn simulate_paste() -> Result<()> {
    use core_graphics::event::{CGEvent, CGEventFlags, CGKeyCode};
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

    const KEY_V: CGKeyCode = 9;

    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|_| anyhow::anyhow!("failed to create event source"))?;

    // Key down with Command modifier
    let key_down = CGEvent::new_keyboard_event(source.clone(), KEY_V, true)
        .map_err(|_| anyhow::anyhow!("failed to create key down event"))?;
    key_down.set_flags(CGEventFlags::CGEventFlagCommand);
    key_down.post(core_graphics::event::CGEventTapLocation::HID);

    // Key up with Command modifier
    let key_up = CGEvent::new_keyboard_event(source, KEY_V, false)
        .map_err(|_| anyhow::anyhow!("failed to create key up event"))?;
    key_up.set_flags(CGEventFlags::CGEventFlagCommand);
    key_up.post(core_graphics::event::CGEventTapLocation::HID);

    Ok(())
}

#[cfg(target_os = "macos")]
fn simulate_return() -> Result<()> {
    use core_graphics::event::CGEvent;
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

    const KEY_RETURN: u16 = 36;

    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|_| anyhow::anyhow!("failed to create event source"))?;

    let key_down = CGEvent::new_keyboard_event(source.clone(), KEY_RETURN, true)
        .map_err(|_| anyhow::anyhow!("failed to create key down event"))?;
    key_down.post(core_graphics::event::CGEventTapLocation::HID);

    let key_up = CGEvent::new_keyboard_event(source, KEY_RETURN, false)
        .map_err(|_| anyhow::anyhow!("failed to create key up event"))?;
    key_up.post(core_graphics::event::CGEventTapLocation::HID);

    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn simulate_paste() -> Result<()> {
    anyhow::bail!("paste simulation only supported on macOS")
}

#[cfg(not(target_os = "macos"))]
fn simulate_return() -> Result<()> {
    anyhow::bail!("return simulation only supported on macOS")
}
