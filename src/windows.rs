use crate::monitor::Monitor;
use log::error;
use std::pin::Pin;
use std::sync::OnceLock;
use windows::Win32::Foundation::{E_FAIL, HWND, LPARAM, LRESULT, MAX_PATH, WPARAM};
use windows::Win32::System::LibraryLoader::{GetModuleFileNameW, GetModuleHandleW};
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::{Error, GUID, PCWSTR, Result, w};
use windows_registry::{CURRENT_USER, HSTRING};

// {F96A5E12-A11C-48E1-9E7D-F38DE6351EF9}
const GUID_SHINOBU_APP: GUID = GUID::from_values(
    0xF96A5E12,
    0xA11C,
    0x48E1,
    [0x9E, 0x7D, 0xF3, 0x8D, 0xE6, 0x35, 0x1E, 0xF9],
);

pub fn main() -> Result<()> {
    MainFrame::register_class()?;

    let mut frame = MainFrame::new()?;
    frame.as_mut().create()?;

    let mut msg = MSG::default();
    loop {
        match unsafe { GetMessageW(&mut msg, None, 0, 0).0 } {
            -1 => break Err(Error::from_thread()),
            0 => break Ok(()),
            _ => unsafe {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            },
        }
    }
}

#[allow(unused)]
enum WM {
    Create(*const CREATESTRUCTW),
    Destroy,
    Command(u16, u16, HWND),
    Timer(usize),
    User(u32, WPARAM, LPARAM),
    App(u32, WPARAM, LPARAM),
    Registered(u32, WPARAM, LPARAM),
    Unknown(u32, WPARAM, LPARAM),
}

impl WM {
    fn crack(msg: u32, wp: WPARAM, lp: LPARAM) -> Self {
        match msg {
            WM_CREATE => Self::Create(lp.0 as _),
            WM_DESTROY => Self::Destroy,
            WM_COMMAND => Self::Command((wp.0 >> 16) as _, wp.0 as _, HWND(lp.0 as _)),
            WM_TIMER => Self::Timer(wp.0),
            WM_USER..=0x7FFF => Self::User(msg, wp, lp),
            WM_APP..=0xBFFF => Self::App(msg, wp, lp),
            0xC000..=0xFFFF => Self::Registered(msg, wp, lp),
            _ => Self::Unknown(msg, wp, lp),
        }
    }
}

static TASKBAR_CREATED: OnceLock<u32> = OnceLock::new();

const ID_APP_EXIT: u16 = 1;
const ID_LAUNCH_AT_LOGIN: u16 = 2;
const ID_PREVENT_DISPLAY_SLEEP: u16 = 3;

struct MainFrame {
    hwnd: HWND,
    monitor: Monitor,
    launch_at_logon: bool,
    require_display: bool,
}

impl MainFrame {
    const CLASS_NAME: PCWSTR = w!("Shinobu");

    fn register_class() -> Result<u16> {
        let instance = unsafe { GetModuleHandleW(None) }.map(Into::into)?;
        let class = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as _,
            lpfnWndProc: Some(Self::window_proc),
            cbWndExtra: size_of::<usize>() as _,
            hInstance: instance,
            lpszClassName: Self::CLASS_NAME,
            ..Default::default()
        };
        match unsafe { RegisterClassExW(&class) } {
            0 => Err(Error::from_thread()),
            atom => Ok(atom),
        }
    }

    fn new() -> Result<Pin<Box<Self>>> {
        let monitor = Monitor::new()?;

        Ok(Box::pin(Self {
            hwnd: <_>::default(),
            monitor,
            launch_at_logon: false,
            require_display: false,
        }))
    }

    fn create(self: Pin<&mut Self>) -> Result<HWND> {
        let instance = unsafe { GetModuleHandleW(None) }.map(Into::into)?;
        unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                Self::CLASS_NAME,
                Self::CLASS_NAME,
                WS_OVERLAPPEDWINDOW,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                Some(HWND_MESSAGE),
                None,
                Some(instance),
                Some(self.get_mut() as *const _ as _),
            )
        }
    }

    fn add_icon(&self) -> Result<()> {
        let instance = unsafe { GetModuleHandleW(None) }.map(Into::into)?;

        let mut tip = [0; 128];
        unsafe { tip[..Self::CLASS_NAME.len()].copy_from_slice(Self::CLASS_NAME.as_wide()) };

        let icon = NOTIFYICONDATAW {
            cbSize: size_of::<NOTIFYICONDATAW>() as _,
            hWnd: self.hwnd,
            uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_SHOWTIP,
            uCallbackMessage: WM_USER,
            hIcon: unsafe { LoadIconW(Some(instance), PCWSTR(1 as _))? },
            szTip: tip,
            Anonymous: NOTIFYICONDATAW_0 {
                uVersion: NOTIFYICON_VERSION_4,
            },
            guidItem: GUID_SHINOBU_APP,
            ..Default::default()
        };

        if unsafe {
            Shell_NotifyIconW(NIM_ADD, &icon).as_bool()
                && Shell_NotifyIconW(NIM_SETVERSION, &icon).as_bool()
        } {
            Ok(())
        } else {
            Err(Error::from_hresult(E_FAIL))
        }
    }

    unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
        unsafe {
            let this_ = if msg == WM_NCCREATE {
                let cs = (lp.0 as *const CREATESTRUCTW).as_ref_unchecked();
                SetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(0), cs.lpCreateParams as _);

                let this_ = cs.lpCreateParams as *mut Self;
                (*this_).hwnd = hwnd;
                this_
            } else {
                GetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(0)) as *mut Self
            };

            match this_.as_mut() {
                Some(this_) => this_.process_message(msg, wp, lp),
                None => DefWindowProcW(hwnd, msg, wp, lp),
            }
        }
    }

    fn process_message(&mut self, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
        let &taskbar_created =
            TASKBAR_CREATED.get_or_init(|| unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) });

        match WM::crack(msg, wp, lp) {
            WM::Create(cs) => {
                if let Err(e) = self.on_create(cs) {
                    error!("{e}");
                    return LRESULT(-1);
                }
            }
            WM::Destroy => self.on_destroy(),
            WM::Command(code, id, control) => self.on_command(code, id, control),
            WM::Timer(id) => self.on_timer(id),
            WM::User(msg, wp, lp) => self.on_tray_notify(msg, wp, lp),
            WM::Registered(msg, wp, lp) if msg == taskbar_created => {
                self.on_taskbar_created(msg, wp, lp)
            }
            _ => return unsafe { DefWindowProcW(self.hwnd, msg, wp, lp) },
        }

        LRESULT(0)
    }

    fn on_create(&mut self, _: *const CREATESTRUCTW) -> Result<()> {
        if let Ok(key) = CURRENT_USER.open("SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run")
            && key.get_string("Shinobu").is_ok()
        {
            self.launch_at_logon = true;
        }

        if let Ok(key) = CURRENT_USER.open("Software\\dacci.org\\Shinobu")
            && let Ok(value) = key.get_u32("PreventDisplaySleep")
        {
            self.require_display = value != 0;
            self.monitor.set_prevent_display_sleep(self.require_display);
        }

        self.add_icon()?;

        Ok(())
    }

    fn on_destroy(&mut self) {
        let icon = NOTIFYICONDATAW {
            cbSize: size_of::<NOTIFYICONDATAW>() as _,
            hWnd: self.hwnd,
            ..Default::default()
        };
        let _ = unsafe { Shell_NotifyIconW(NIM_DELETE, &icon) };

        unsafe { PostQuitMessage(0) };
    }

    fn on_command(&mut self, _code: u16, id: u16, _control: HWND) {
        match id {
            ID_LAUNCH_AT_LOGIN => {
                let key = match CURRENT_USER
                    .create("SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run")
                {
                    Ok(key) => key,
                    Err(e) => {
                        error!("Failed to create/open key: {e}");
                        return;
                    }
                };

                if self.launch_at_logon {
                    if let Err(e) = key.remove_value("Shinobu") {
                        error!("Faield to remove value: {e}");
                        return;
                    }
                } else {
                    let path = {
                        let mut path = [0; MAX_PATH as _];
                        let len = unsafe { GetModuleFileNameW(None, &mut path) } as usize;

                        let mut path = Vec::from(&path[..len]);
                        path.insert(0, '"' as _);
                        path.push('"' as _);
                        HSTRING::from_wide(&path)
                    };

                    if let Err(e) = key.set_hstring("Shinobu", &path) {
                        error!("Failed to write value: {e}");
                        return;
                    }
                }

                self.launch_at_logon = !self.launch_at_logon;
            }
            ID_PREVENT_DISPLAY_SLEEP => {
                let require_display = !self.require_display;

                let key = match CURRENT_USER.create("Software\\dacci.org\\Shinobu") {
                    Ok(key) => key,
                    Err(e) => {
                        error!("Failed to create/open key: {e}");
                        return;
                    }
                };
                if let Err(e) = key.set_u32("PreventDisplaySleep", require_display as _) {
                    error!("Failed to write value: {e}");
                    return;
                };

                self.require_display = require_display;
                self.monitor.set_prevent_display_sleep(require_display)
            }
            ID_APP_EXIT => {
                let _ = unsafe { DestroyWindow(self.hwnd) };
            }
            _ => (),
        }
    }

    fn on_timer(&mut self, _id: usize) {
        self.monitor.tick();
    }

    fn on_tray_notify(&mut self, _msg: u32, wp: WPARAM, lp: LPARAM) {
        if lp.0 == WM_CONTEXTMENU as _ {
            let (x, y) = ((wp.0 & 0xFFFF) as i32, (wp.0 >> 16) as i32);
            unsafe {
                let menu = CreatePopupMenu().unwrap();

                let _ = AppendMenuW(
                    menu,
                    if self.launch_at_logon {
                        MF_CHECKED
                    } else {
                        MF_UNCHECKED
                    },
                    ID_LAUNCH_AT_LOGIN as _,
                    w!("Launch at login"),
                );
                let _ = AppendMenuW(
                    menu,
                    if self.require_display {
                        MF_CHECKED
                    } else {
                        MF_UNCHECKED
                    },
                    ID_PREVENT_DISPLAY_SLEEP as _,
                    w!("Prevent display sleep"),
                );
                let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);
                let _ = AppendMenuW(menu, MF_STRING, ID_APP_EXIT as _, w!("E&xit"));

                let _ = SetForegroundWindow(self.hwnd);
                let _ = TrackPopupMenu(menu, TPM_RIGHTBUTTON, x, y, None, self.hwnd, None);
                let _ = PostMessageW(Some(self.hwnd), WM_NULL, WPARAM(0), LPARAM(0));

                let _ = DestroyMenu(menu);
            }
        }
    }

    fn on_taskbar_created(&mut self, _msg: u32, _wp: WPARAM, _lp: LPARAM) {
        let _ = self.add_icon();
    }
}
