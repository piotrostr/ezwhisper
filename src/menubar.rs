#![allow(unexpected_cfgs, deprecated)]

#[cfg(target_os = "macos")]
use cocoa::appkit::{
    NSApp, NSApplication, NSApplicationActivationPolicy, NSMenu, NSMenuItem, NSStatusBar,
    NSVariableStatusItemLength,
};
#[cfg(target_os = "macos")]
use cocoa::base::{id, nil, NO};
#[cfg(target_os = "macos")]
use cocoa::foundation::{NSAutoreleasePool, NSDefaultRunLoopMode, NSString};
#[cfg(target_os = "macos")]
use objc::runtime::Sel;
#[cfg(target_os = "macos")]
use objc::{class, msg_send, sel, sel_impl};

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppStatus {
    Idle,
    Recording,
    Transcribing,
}

#[allow(dead_code)]
pub enum MenuCommand {
    SelectDevice(usize),
    Quit,
}

#[cfg(target_os = "macos")]
pub struct MenuBar {
    status_item: id,
    menu: id,
    #[allow(dead_code)]
    command_tx: mpsc::Sender<MenuCommand>,
    device_names: Arc<Mutex<Vec<String>>>,
    selected_device: Arc<AtomicUsize>,
}

#[cfg(target_os = "macos")]
impl MenuBar {
    pub fn new(command_tx: mpsc::Sender<MenuCommand>) -> Self {
        unsafe {
            let _pool = NSAutoreleasePool::new(nil);

            let app = NSApp();
            app.setActivationPolicy_(NSApplicationActivationPolicy::NSApplicationActivationPolicyAccessory);

            let status_bar = NSStatusBar::systemStatusBar(nil);
            let status_item = status_bar.statusItemWithLength_(NSVariableStatusItemLength);

            let button: id = msg_send![status_item, button];
            let title = NSString::alloc(nil).init_str("EZ");
            let _: () = msg_send![button, setTitle: title];

            let menu = NSMenu::new(nil).autorelease();

            Self {
                status_item,
                menu,
                command_tx,
                device_names: Arc::new(Mutex::new(Vec::new())),
                selected_device: Arc::new(AtomicUsize::new(0)),
            }
        }
    }

    pub fn set_status(&self, status: AppStatus) {
        unsafe {
            let button: id = msg_send![self.status_item, button];
            let title = match status {
                AppStatus::Idle => "EZ",
                AppStatus::Recording => "R",
                AppStatus::Transcribing => "T",
            };
            let ns_title = NSString::alloc(nil).init_str(title);
            let _: () = msg_send![button, setTitle: ns_title];
        }
    }

    /// Pump the run loop to allow UI updates
    pub fn pump(&self) {
        unsafe {
            let _pool = NSAutoreleasePool::new(nil);
            let run_loop: id = msg_send![class!(NSRunLoop), currentRunLoop];
            let date: id = msg_send![class!(NSDate), dateWithTimeIntervalSinceNow: 0.0001f64];
            let _: () = msg_send![run_loop, runMode: NSDefaultRunLoopMode beforeDate: date];
        }
    }

    pub fn set_devices(&self, devices: Vec<String>) {
        let selected = self.selected_device.load(Ordering::SeqCst);
        {
            let mut names = self.device_names.lock().unwrap();
            *names = devices.clone();
        }
        self.rebuild_menu(&devices, selected);
    }

    pub fn set_selected_device(&self, index: usize) {
        self.selected_device.store(index, Ordering::SeqCst);
        let devices = self.device_names.lock().unwrap().clone();
        self.rebuild_menu(&devices, index);
    }

    fn rebuild_menu(&self, devices: &[String], selected: usize) {
        unsafe {
            let _pool = NSAutoreleasePool::new(nil);

            // Clear existing menu
            let _: () = msg_send![self.menu, removeAllItems];

            // Status item (disabled, just for display)
            let status_title = NSString::alloc(nil).init_str("Status: Ready");
            let status_item = NSMenuItem::alloc(nil).initWithTitle_action_keyEquivalent_(
                status_title,
                Sel::from_ptr(std::ptr::null()),
                NSString::alloc(nil).init_str(""),
            );
            let _: () = msg_send![status_item, setEnabled: NO];
            self.menu.addItem_(status_item);

            // Separator
            let separator: id = msg_send![class!(NSMenuItem), separatorItem];
            self.menu.addItem_(separator);

            // Audio Input header
            let header_title = NSString::alloc(nil).init_str("Audio Input:");
            let header_item = NSMenuItem::alloc(nil).initWithTitle_action_keyEquivalent_(
                header_title,
                Sel::from_ptr(std::ptr::null()),
                NSString::alloc(nil).init_str(""),
            );
            let _: () = msg_send![header_item, setEnabled: NO];
            self.menu.addItem_(header_item);

            // Device items
            for (i, name) in devices.iter().enumerate() {
                let display_name = if i == selected {
                    format!("  {} {}", "\u{2713}", name)
                } else {
                    format!("     {}", name)
                };

                let item_title = NSString::alloc(nil).init_str(&display_name);
                let item = NSMenuItem::alloc(nil).initWithTitle_action_keyEquivalent_(
                    item_title,
                    sel!(deviceSelected:),
                    NSString::alloc(nil).init_str(""),
                );
                let _: () = msg_send![item, setTag: i as isize];
                let _: () = msg_send![item, setTarget: self.status_item];
                self.menu.addItem_(item);
            }

            // Separator
            let separator2: id = msg_send![class!(NSMenuItem), separatorItem];
            self.menu.addItem_(separator2);

            // Quit item
            let quit_title = NSString::alloc(nil).init_str("Quit");
            let quit_item = NSMenuItem::alloc(nil).initWithTitle_action_keyEquivalent_(
                quit_title,
                sel!(terminate:),
                NSString::alloc(nil).init_str("q"),
            );
            self.menu.addItem_(quit_item);

            // Attach menu to status item
            let _: () = msg_send![self.status_item, setMenu: self.menu];
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub struct MenuBar;

#[cfg(not(target_os = "macos"))]
impl MenuBar {
    pub fn new(_command_tx: mpsc::Sender<MenuCommand>) -> Self {
        Self
    }

    pub fn set_status(&self, _status: AppStatus) {}

    pub fn pump(&self) {}

    pub fn set_devices(&self, _devices: Vec<String>) {}

    pub fn set_selected_device(&self, _index: usize) {}
}
