use anyhow::Result;
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
            run_cg_event_tap(tx);
        });

        Ok(Self { event_rx: rx })
    }

    pub fn try_recv(&self) -> Option<InputEvent> {
        self.event_rx.try_recv().ok()
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use super::*;
    use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop};
    use core_graphics::event::{CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::OnceLock;

    // Link to CoreGraphics and CoreFoundation frameworks
    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGEventTapCreate(
            tap: u32,
            place: u32,
            options: u32,
            events_of_interest: u64,
            callback: extern "C" fn(
                *mut std::ffi::c_void,
                u32,
                *mut std::ffi::c_void,
                *mut std::ffi::c_void,
            ) -> *mut std::ffi::c_void,
            user_info: *mut std::ffi::c_void,
        ) -> *mut std::ffi::c_void;

        fn CGEventTapEnable(tap: *mut std::ffi::c_void, enable: bool);

        fn CGEventGetIntegerValueField(event: *mut std::ffi::c_void, field: u32) -> i64;
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn CFMachPortCreateRunLoopSource(
            allocator: *const std::ffi::c_void,
            port: *mut std::ffi::c_void,
            order: i64,
        ) -> *mut std::ffi::c_void;

        fn CFRunLoopAddSource(
            rl: *mut std::ffi::c_void,
            source: *mut std::ffi::c_void,
            mode: *const std::ffi::c_void,
        );

        fn CFRunLoopGetCurrent() -> *mut std::ffi::c_void;

        fn CFRunLoopRun();
    }

    // Logitech gesture button sends keycode 65535 (0xFFFF)
    const LOGITECH_GESTURE_KEYCODE: i64 = 65535;

    // Right Option key (kVK_RightOption = 0x3D = 61)
    const RIGHT_OPTION_KEYCODE: i64 = 61;

    // Mouse button codes for Logitech buttons (side buttons)
    const TRIGGER_MOUSE_BUTTONS: [i64; 5] = [3, 4, 5, 6, 8];

    // Event type constants
    const KEY_DOWN: u32 = 10;
    const KEY_UP: u32 = 11;
    const OTHER_MOUSE_DOWN: u32 = 25;
    const OTHER_MOUSE_UP: u32 = 26;

    // Event field constants
    const KEYBOARD_EVENT_KEYCODE: u32 = 9;
    const MOUSE_EVENT_BUTTON_NUMBER: u32 = 3;

    // Use static for callback state since CGEventTap callback must be extern "C"
    static TX: OnceLock<Sender<InputEvent>> = OnceLock::new();
    static IS_PRESSED: AtomicBool = AtomicBool::new(false);

    extern "C" fn callback(
        _proxy: *mut std::ffi::c_void,
        event_type: u32,
        event: *mut std::ffi::c_void,
        _user_info: *mut std::ffi::c_void,
    ) -> *mut std::ffi::c_void {
        let Some(tx) = TX.get() else {
            return event;
        };

        unsafe {
            match event_type {
                KEY_DOWN => {
                    let keycode = CGEventGetIntegerValueField(event, KEYBOARD_EVENT_KEYCODE);
                    let is_trigger = keycode == LOGITECH_GESTURE_KEYCODE || keycode == RIGHT_OPTION_KEYCODE;
                    tracing::info!("KEY_DOWN keycode: {} (trigger={})", keycode, is_trigger);
                    if is_trigger {
                        // Always send on key down - toggle mode handles state
                        let _ = tx.send(InputEvent::TriggerPressed);
                    }
                }
                KEY_UP => {
                    let keycode = CGEventGetIntegerValueField(event, KEYBOARD_EVENT_KEYCODE);
                    tracing::info!("KEY_UP keycode: {}", keycode);
                    // We don't use KEY_UP in toggle mode
                }
                OTHER_MOUSE_DOWN => {
                    let button = CGEventGetIntegerValueField(event, MOUSE_EVENT_BUTTON_NUMBER);
                    if TRIGGER_MOUSE_BUTTONS.contains(&button) && !IS_PRESSED.load(Ordering::SeqCst)
                    {
                        IS_PRESSED.store(true, Ordering::SeqCst);
                        let _ = tx.send(InputEvent::TriggerPressed);
                    }
                }
                OTHER_MOUSE_UP => {
                    let button = CGEventGetIntegerValueField(event, MOUSE_EVENT_BUTTON_NUMBER);
                    if TRIGGER_MOUSE_BUTTONS.contains(&button) && IS_PRESSED.load(Ordering::SeqCst)
                    {
                        IS_PRESSED.store(false, Ordering::SeqCst);
                        let _ = tx.send(InputEvent::TriggerReleased);
                    }
                }
                _ => {}
            }
        }

        event
    }

    pub fn run(tx: Sender<InputEvent>) {
        TX.set(tx).ok();

        // Event mask for keyboard and mouse events
        let event_mask: u64 = (1 << KEY_DOWN)
            | (1 << KEY_UP)
            | (1 << OTHER_MOUSE_DOWN)
            | (1 << OTHER_MOUSE_UP);

        unsafe {
            let tap = CGEventTapCreate(
                CGEventTapLocation::HID as u32,
                CGEventTapPlacement::HeadInsertEventTap as u32,
                CGEventTapOptions::ListenOnly as u32,
                event_mask,
                callback,
                std::ptr::null_mut(),
            );

            if tap.is_null() {
                tracing::error!("failed to create event tap - check Input Monitoring permission");
                return;
            }

            let source = CFMachPortCreateRunLoopSource(std::ptr::null(), tap, 0);
            if source.is_null() {
                tracing::error!("failed to create run loop source");
                return;
            }

            let run_loop = CFRunLoopGetCurrent();
            CFRunLoopAddSource(
                run_loop,
                source,
                kCFRunLoopCommonModes as *const _ as *const std::ffi::c_void,
            );

            CGEventTapEnable(tap, true);

            tracing::info!("input monitoring started (CGEventTap)");

            CFRunLoopRun();
        }
    }
}

#[cfg(target_os = "macos")]
fn run_cg_event_tap(tx: Sender<InputEvent>) {
    macos::run(tx);
}

#[cfg(not(target_os = "macos"))]
fn run_cg_event_tap(_tx: Sender<InputEvent>) {
    tracing::error!("CGEventTap only supported on macOS");
}
