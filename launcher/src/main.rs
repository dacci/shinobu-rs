#![allow(clippy::let_unit_value)]

use cocoa::appkit::{
    NSApp, NSApplication, NSEvent, NSEventModifierFlags, NSEventSubtype, NSEventType, NSWindow,
};
use cocoa::base::{id, nil};
use cocoa::delegate;
use cocoa::foundation::{NSArray, NSPoint, NSString};
use objc::runtime::{Object, Sel, BOOL, NO, YES};
use objc::{class, msg_send, sel, sel_impl};

fn main() {
    unsafe {
        let app = NSApp();

        let app_delegate = delegate!("AppDelegate", {
            app: id = app,
            (applicationDidFinishLaunching:) => application_did_finish_launching as extern fn (&mut Object, Sel, id)
        });
        app.setDelegate_(app_delegate);

        app.run();
    }
}

extern "C" fn application_did_finish_launching(this: &mut Object, _: Sel, _: id) {
    unsafe {
        launch_if_not_running();

        let app: &id = this.get_ivar("app");
        app.stop_(nil);

        let event = NSEvent::otherEventWithType_location_modifierFlags_timestamp_windowNumber_context_subtype_data1_data2_(
            nil,
            NSEventType::NSApplicationDefined,
            NSPoint::new(0.0, 0.0),
            NSEventModifierFlags::empty(),
            0.0,
            0,
            nil,
            NSEventSubtype::NSWindowExposedEventType,
            0,
            0);
        app.postEvent_atStart_(event, YES);
    }
}

unsafe fn launch_if_not_running() {
    let bundle_id = NSString::alloc(nil).init_str("org.dacci.shinobu");

    let workspace: id = msg_send![class!(NSWorkspace), sharedWorkspace];
    if is_running(workspace, bundle_id) {
        return;
    }

    let url: id = msg_send![workspace, URLForApplicationWithBundleIdentifier: bundle_id];
    let config: id = msg_send![class!(NSWorkspaceOpenConfiguration), alloc];
    let _: () =
        msg_send![workspace, openApplicationAtURL:url configuration:config completionHandler:nil];

    // FIXME(dacci): use completionHandler
    std::thread::sleep(std::time::Duration::from_secs(1));
}

unsafe fn is_running(workspace: id, bundle_id: id) -> bool {
    let apps: id = msg_send![workspace, runningApplications];

    for i in 0..apps.count() {
        let app = apps.objectAtIndex(i);
        let app_id: id = msg_send![app, bundleIdentifier];
        let equals: BOOL = msg_send![app_id, isEqualToString: bundle_id];
        if equals != NO {
            return true;
        }
    }

    false
}
