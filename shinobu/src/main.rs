#![allow(clippy::let_unit_value)]

mod data;
mod sys;

use anyhow::Result;
use cocoa::appkit::{
    NSApp, NSApplication, NSApplicationActivationPolicyAccessory, NSMenu, NSSquareStatusItemLength,
    NSStatusBar, NSStatusItem, NSWindow,
};
use cocoa::base::{id, nil, selector};
use cocoa::foundation::NSString;
use core_foundation::base::TCFType;
use core_foundation::string::CFString;
use core_foundation_sys::base::Boolean;
use core_foundation_sys::string::CFStringRef;
use data::Historical;
use futures::StreamExt;
use objc::runtime::{objc_retain, Object, Sel, BOOL, NO, YES};
use objc::{class, msg_send, sel, sel_impl};
use std::collections::HashMap;
use tokio::time::{interval, Duration, MissedTickBehavior};

fn main() -> Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    rt.spawn(worker());

    unsafe {
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
    }

    Ok(())
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
    #[link(name = "ServiceManagement", kind = "framework")]
    extern "system" {
        fn SMLoginItemSetEnabled(identifier: CFStringRef, enabled: Boolean) -> Boolean;
    }

    unsafe {
        let user_defaults: id = msg_send![class!(NSUserDefaults), standardUserDefaults];
        let key = NSString::alloc(nil).init_str("launchAtLogin");
        let enabled: BOOL = msg_send![user_defaults, boolForKey: key];

        let identifier = CFString::from_static_string("org.dacci.shinobu.launcher");
        let new_value = match enabled {
            NO => 1,
            _ => 0,
        };
        if SMLoginItemSetEnabled(identifier.as_concrete_TypeRef(), new_value) == 1 {
            let _: () = msg_send![user_defaults, setBool:!enabled forKey:key];
        }
    }
}

const NET_THRESHOLD: f64 = 50.0 * 1024.0;
const MA_LENGTH: usize = 300;

async fn worker() -> Result<()> {
    let inhibitor = sys::Inhibitor::new().await?;
    let mut assertion = None;

    let monitor = sys::net::Monitor::new().await?;

    let mut if_hist = HashMap::new();
    let mut hist_in = Historical::new();
    let mut hist_out = Historical::new();

    let mut interval = interval(Duration::from_secs(1));
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        interval.tick().await;

        let mut diff_in = 0;
        let mut diff_out = 0;

        let stats = match monitor.current().await {
            Ok(stats) => stats,
            Err(e) => {
                eprintln!("{e}");
                continue;
            }
        };
        tokio::pin!(stats);
        while let Some(stat) = stats.next().await {
            let stat = match stat {
                Ok(stat) => stat,
                Err(e) => {
                    eprintln!("{e}");
                    continue;
                }
            };

            let (hist_in, hist_out) = if_hist
                .entry(stat.name)
                .or_insert_with_key(|_| (Historical::new(), Historical::new()));
            if let Some(diff) = hist_in.push(stat.in_bytes) {
                diff_in += diff;
            }
            if let Some(diff) = hist_out.push(stat.out_bytes) {
                diff_out += diff;
            }
        }

        hist_in.push_diff(diff_in);
        hist_out.push_diff(diff_out);

        let medium = hist_in.moving_average(MA_LENGTH) + hist_out.moving_average(MA_LENGTH);
        if medium < NET_THRESHOLD && assertion.is_some() {
            assertion = None;
        } else if NET_THRESHOLD <= medium && assertion.is_none() {
            assertion = Some(inhibitor.inhibit().await);
        }
    }
}
