use cocoa::appkit::{
    NSApp, NSApplication, NSApplicationActivationPolicyAccessory, NSMenu, NSSquareStatusItemLength,
    NSStatusBar, NSStatusItem, NSWindow,
};
use cocoa::base::{id, nil, selector};
use cocoa::foundation::NSString;
use objc::rc::autoreleasepool;
use objc::runtime::{objc_retain, Object, Sel, BOOL, NO, YES};
use objc::{class, msg_send, sel, sel_impl};
use std::ptr::null_mut;

pub(super) fn main() {
    autoreleasepool(|| unsafe {
        let app = NSApp();

        let app_delegate = cocoa::delegate!("AppDelegate", {
            app: id = app,
            status_item: id = nil,
            (applicationDidFinishLaunching:) => application_did_finish_launching as extern fn (&mut Object, Sel, id),
            (validateMenuItem:) => validate_menu_item as extern fn (&mut Object, Sel, id) -> BOOL,
            (toggleLaunchAtLogin:) => toggle_launch_at_login as extern fn (&mut Object, Sel, id)
        });
        app.setDelegate_(app_delegate);

        app.run();
    })
}

extern "C" fn application_did_finish_launching(this: &mut Object, _: Sel, _: id) {
    unsafe {
        let status_menu = NSMenu::alloc(nil);

        let title = NSString::alloc(nil).init_str("Launch at login");
        let key = NSString::alloc(nil).init_str("");
        status_menu.addItemWithTitle_action_keyEquivalent(
            title,
            selector("toggleLaunchAtLogin:"),
            key,
        );

        let quit_title = NSString::alloc(nil).init_str("Quit Shinobu");
        let quit_key = NSString::alloc(nil).init_str("");
        status_menu.addItemWithTitle_action_keyEquivalent(quit_title, selector("stop:"), quit_key);

        let status_item =
            NSStatusBar::systemStatusBar(nil).statusItemWithLength_(NSSquareStatusItemLength);
        let title = NSString::alloc(nil).init_str("忍");
        status_item.button().setTitle_(title);
        status_item.setMenu_(status_menu);
        this.set_ivar("status_item", objc_retain(status_item));

        let app: &id = this.get_ivar("app");
        app.setActivationPolicy_(NSApplicationActivationPolicyAccessory);
        app.activateIgnoringOtherApps_(NO);
    }
}

extern "C" fn validate_menu_item(_: &mut Object, _: Sel, item: id) -> BOOL {
    unsafe {
        let action: Sel = msg_send![item, action];
        if action.name() == "toggleLaunchAtLogin:" {
            let user_defaults: id = msg_send![class!(NSUserDefaults), standardUserDefaults];
            let key = NSString::alloc(nil).init_str("launchAtLogin");
            let state = match msg_send![user_defaults, boolForKey: key] {
                NO => 0,
                _ => 1,
            };
            let _: () = msg_send![item, setState: state];
        }
    }

    YES
}

extern "C" fn toggle_launch_at_login(_: &mut Object, _: Sel, _: id) {
    unsafe {
        let user_defaults: id = msg_send![class!(NSUserDefaults), standardUserDefaults];
        let key = NSString::alloc(nil).init_str("launchAtLogin");
        let enabled: BOOL = msg_send![user_defaults, boolForKey: key];

        let service: id = msg_send![class!(SMAppService), mainAppService];

        let succeeded: BOOL = if enabled == NO {
            msg_send![service, registerAndReturnError: null_mut::<id>()]
        } else {
            msg_send![service, unregisterAndReturnError: null_mut::<id>()]
        };

        if succeeded != NO {
            let new_value = match enabled {
                NO => YES,
                _ => NO,
            };
            let _: () = msg_send![user_defaults, setBool:new_value forKey:key];
        }
    }
}
