use crate::monitor::Monitor;
use objc2::rc::Retained;
use objc2::runtime::NSObjectProtocol;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{
    NSAlert, NSAlertStyle, NSApplication, NSApplicationActivationPolicy, NSApplicationDelegate,
    NSControlStateValueOff, NSControlStateValueOn, NSMenu, NSMenuItem, NSMenuItemValidation,
    NSSquareStatusItemLength, NSStatusBar, NSStatusItem,
};
use objc2_foundation::{NSNotification, NSObject, NSTimer, NSUserDefaults, ns_string};
use objc2_service_management::SMAppService;
use std::cell::RefCell;

#[derive(Default)]
pub struct AppDelegateIvar {
    monitor: RefCell<Option<Monitor>>,
    status_item: RefCell<Option<Retained<NSStatusItem>>>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = AppDelegateIvar]
    pub struct AppDelegate;

    unsafe impl NSObjectProtocol for AppDelegate {}

    unsafe impl NSApplicationDelegate for AppDelegate {
        #[unsafe(method(applicationDidFinishLaunching:))]
        fn did_finish_launching(&self, notification: &NSNotification) {
            let defaults = NSUserDefaults::standardUserDefaults();

            let monitor = Monitor::new();
            monitor.set_prevent_display_sleep(defaults.boolForKey(ns_string!("preventDisplaySleep")));
            self.ivars().monitor.replace(Some(monitor));

            unsafe {
                NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
                    1.0,
                    self,
                    sel!(timerFired:),
                    None,
                    true,
                )
            };

            let status_menu = NSMenu::new(self.mtm());
            unsafe {
                status_menu.addItemWithTitle_action_keyEquivalent(
                    ns_string!("Launch at login"),
                    Some(sel!(toggleLaunchAtLogin:)),
                    ns_string!(""),
                );
                status_menu.addItemWithTitle_action_keyEquivalent(
                    ns_string!("Prevent display sleep"),
                    Some(sel!(togglePreventDisplaySleep:)),
                    ns_string!(""),
                );
                status_menu.addItem(NSMenuItem::separatorItem(self.mtm()).as_ref());
                status_menu.addItemWithTitle_action_keyEquivalent(
                    ns_string!("Quit Shinobu"),
                    Some(sel!(terminate:)),
                    ns_string!(""),
                );
            }

            let status_item =
                NSStatusBar::systemStatusBar().statusItemWithLength(NSSquareStatusItemLength);
            status_item
                .button(self.mtm())
                .unwrap()
                .setTitle(ns_string!("忍"));
            status_item.setMenu(Some(status_menu.as_ref()));
            self.ivars().status_item.replace(Some(status_item));

            let app = notification
                .object()
                .unwrap()
                .downcast::<NSApplication>()
                .unwrap();
            app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
        }
    }

    unsafe impl NSMenuItemValidation for AppDelegate {
        #[unsafe(method(validateMenuItem:))]
        fn validate_menu_item(&self, item: &NSMenuItem) -> bool {
            let Some(action) = item.action() else { return false.into() };

            let defaults = NSUserDefaults::standardUserDefaults();

            if action.name() == c"toggleLaunchAtLogin:" {
                let state = if defaults.boolForKey(ns_string!("launchAtLogin")) {
                    NSControlStateValueOn
                } else {
                    NSControlStateValueOff
                };
                item.setState(state);
                true
            } else if action.name() == c"togglePreventDisplaySleep:" {
                let state = if defaults.boolForKey(ns_string!("preventDisplaySleep")) {
                    NSControlStateValueOn
                } else {
                    NSControlStateValueOff
                };
                item.setState(state);
                true
            } else {
                false
            }
        }
    }

    impl AppDelegate {
        #[unsafe(method(toggleLaunchAtLogin:))]
        fn toggle_launch_at_login(&self, _: &NSNotification) {
            let key = ns_string!("launchAtLogin");

            let user_defaults = NSUserDefaults::standardUserDefaults();
            let enabled = user_defaults.boolForKey(key);

            let result = unsafe {
                let service = SMAppService::mainAppService();
                if enabled {
                    service.unregisterAndReturnError()
                } else {
                    service.registerAndReturnError()
                }
            };
            match result {
                Ok(_) => user_defaults.setBool_forKey(!enabled, key),
                Err(e) => {
                    let alert = NSAlert::new(self.mtm());
                    alert.setAlertStyle(NSAlertStyle::Critical);
                    alert.setMessageText(ns_string!("Error"));
                    alert.setInformativeText(e.localizedDescription().as_ref());
                    alert.addButtonWithTitle(ns_string!("OK"));
                    alert.runModal();
                }
            }
        }

        #[unsafe(method(togglePreventDisplaySleep:))]
        fn toggle_prevent_display_sleep(&self, _: &NSNotification) {
            let user_defaults = NSUserDefaults::standardUserDefaults();

            let key = ns_string!("preventDisplaySleep");
            let enabled = user_defaults.boolForKey(key);
            user_defaults.setBool_forKey(!enabled, key);

            self.ivars()
                .monitor
                .borrow()
                .as_ref()
                .unwrap()
                .set_prevent_display_sleep(!enabled);
        }

        #[unsafe(method(timerFired:))]
        fn timer_fired(&self, _: &NSTimer) {
            self.ivars().monitor.borrow().as_ref().unwrap().tick();
        }
    }
);

impl AppDelegate {
    pub fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(<_>::default());
        unsafe { msg_send![super(this), init] }
    }
}
