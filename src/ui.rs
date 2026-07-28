use objc2::rc::Retained;
use objc2::runtime::{NSObjectProtocol, ProtocolObject};
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{
    NSAlert, NSAlertStyle, NSApplication, NSApplicationActivationPolicy, NSApplicationDelegate,
    NSControlStateValueOff, NSControlStateValueOn, NSMenu, NSMenuItem, NSMenuItemValidation,
    NSSquareStatusItemLength, NSStatusBar, NSStatusItem,
};
use objc2_foundation::{NSNotification, NSObject, NSUserDefaults, ns_string};
use objc2_service_management::SMAppService;
use std::cell::RefCell;

pub(super) fn main() {
    let mtm = MainThreadMarker::new().unwrap();
    let app = NSApplication::sharedApplication(mtm);

    let delegate = AppDelegate::new(mtm);
    let object = ProtocolObject::from_ref(&*delegate);
    app.setDelegate(Some(object));

    app.run();
}

#[derive(Default)]
struct AppDelegateIvar {
    status_item: RefCell<Retained<NSStatusItem>>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = AppDelegateIvar]
    struct AppDelegate;

    unsafe impl NSObjectProtocol for AppDelegate {}

    unsafe impl NSApplicationDelegate for AppDelegate {
        #[unsafe(method(applicationDidFinishLaunching:))]
        fn did_finish_launching(&self, notification: &NSNotification) {
            let status_menu = NSMenu::new(self.mtm());
            unsafe {
                status_menu.addItemWithTitle_action_keyEquivalent(
                    ns_string!("Launch at login"),
                    Some(sel!(toggleLaunchAtLogin:)),
                    ns_string!(""),
                );
                status_menu.addItemWithTitle_action_keyEquivalent(
                    ns_string!("Quit Shinobu"),
                    Some(sel!(stop:)),
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
            self.ivars().status_item.replace(status_item);

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

            if action.name() == c"toggleLaunchAtLogin:" {
                let defaults = NSUserDefaults::standardUserDefaults();
                let state = if defaults.boolForKey(ns_string!("launchAtLogin")) {
                    NSControlStateValueOn
                } else {
                    NSControlStateValueOff
                };
                item.setState(state);
            }

            true
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
    }
);

impl AppDelegate {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(<_>::default());
        unsafe { msg_send![super(this), init] }
    }
}
