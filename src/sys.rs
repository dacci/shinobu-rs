use objc2_core_foundation::CFString;
use objc2_io_kit::{IOPMAssertionCreateWithDescription, IOPMAssertionID, IOPMAssertionRelease};
use std::io::{Error, Result};

pub struct Inhibitor;

impl Inhibitor {
    pub fn new() -> Self {
        Self
    }

    pub fn inhibit(&self) -> Result<Assertion> {
        let mut assertion_id = 0;
        let ret = unsafe {
            IOPMAssertionCreateWithDescription(
                Some(
                    CFString::from_str(iokit::kIOPMAssertionTypePreventUserIdleDisplaySleep)
                        .as_ref(),
                ),
                Some(CFString::from_str("Shinobu").as_ref()),
                Some(CFString::from_str("System activity").as_ref()),
                Some(CFString::from_str("Shinobu is preventing sleep.").as_ref()),
                Some(CFString::from_str(iokit::kLocalizationBundlePath).as_ref()),
                0.0,
                Some(CFString::from_str(iokit::kIOPMAssertionTimeoutActionRelease).as_ref()),
                &mut assertion_id,
            )
        };
        match ret {
            0 => Ok(Assertion(assertion_id)),
            code => Err(Error::from_raw_os_error(code)),
        }
    }
}

pub struct Assertion(IOPMAssertionID);

impl Drop for Assertion {
    fn drop(&mut self) {
        IOPMAssertionRelease(self.0);
    }
}

mod iokit {
    #![allow(unused, non_upper_case_globals)]

    pub const kIOPMAssertionTypePreventUserIdleSystemSleep: &str = "PreventUserIdleSystemSleep";
    pub const kIOPMAssertionTypePreventUserIdleDisplaySleep: &str = "PreventUserIdleDisplaySleep";
    pub const kIOPMAssertionTypePreventSystemSleep: &str = "PreventSystemSleep";
    pub const kIOPMAssertionUserIsActive: &str = "UserIsActive";
    pub const kIOPMAssertPreventDiskIdle: &str = "PreventDiskIdle";

    pub const kLocalizationBundlePath: &str = "/System/Library/CoreServices/powerd.bundle";

    pub const kIOPMAssertionTimeoutActionLog: &str = "TimeoutActionLog";
    pub const kIOPMAssertionTimeoutActionTurnOff: &str = "TimeoutActionTurnOff";
    pub const kIOPMAssertionTimeoutActionRelease: &str = "TimeoutActionRelease";
}

pub mod net {
    use std::io::{Error, Result};
    use std::marker::PhantomData;
    use std::ptr::null_mut;

    pub struct Monitor;

    impl Monitor {
        pub fn new() -> Self {
            Self
        }

        pub fn current(&self) -> Result<impl Iterator<Item = Result<Stat>>> {
            let mut name = [libc::CTL_NET, libc::PF_ROUTE, 0, 0, libc::NET_RT_IFLIST2, 0];
            let mut needed = 0;
            let res = unsafe {
                libc::sysctl(
                    name.as_mut_ptr(),
                    name.len() as _,
                    null_mut(),
                    &mut needed,
                    null_mut(),
                    0,
                )
            };
            if res != 0 {
                return Err(Error::last_os_error());
            }

            let mut buf = vec![0; needed];
            let res = unsafe {
                libc::sysctl(
                    name.as_mut_ptr(),
                    name.len() as _,
                    buf.as_mut_ptr() as _,
                    &mut needed,
                    null_mut(),
                    0,
                )
            };
            if res != 0 {
                return Err(Error::last_os_error());
            }

            Ok(Routes {
                buf,
                pos: 0,
                _a: PhantomData,
            }
            .map(Stat::try_from))
        }
    }

    struct Routes<'a> {
        buf: Vec<u8>,
        pos: usize,
        _a: PhantomData<&'a ()>,
    }

    impl<'a> Iterator for Routes<'a> {
        type Item = &'a libc::if_msghdr2;

        fn next(&mut self) -> Option<Self::Item> {
            #[inline]
            fn has_flag(flags: libc::c_int, bits: libc::c_int) -> bool {
                flags & bits == bits
            }

            while self.pos < self.buf.len() {
                let msg = unsafe { self.buf.as_ptr().add(self.pos) } as *const libc::if_msghdr2;
                let msg = unsafe { msg.as_ref() }.unwrap();

                if msg.ifm_msglen == 0 {
                    break;
                }
                self.pos += msg.ifm_msglen as usize;

                if msg.ifm_type != libc::RTM_IFINFO2 as _ {
                    continue;
                }

                if has_flag(msg.ifm_flags, libc::IFF_LOOPBACK) {
                    continue;
                }

                return Some(msg);
            }

            None
        }
    }

    #[allow(unused)]
    pub struct Stat {
        pub name: String,
        pub flags: u32,
        pub in_bytes: u64,
        pub out_bytes: u64,
    }

    impl TryFrom<&libc::if_msghdr2> for Stat {
        type Error = Error;

        fn try_from(msg: &libc::if_msghdr2) -> Result<Self> {
            let name = {
                let mut name = [0u8; libc::IF_NAMESIZE];
                let ret =
                    unsafe { libc::if_indextoname(msg.ifm_index as _, name.as_mut_ptr() as _) };
                if ret.is_null() {
                    return Err(Error::last_os_error());
                }

                let len = name.iter().position(|c| *c == 0).unwrap();
                String::from_utf8_lossy(&name[..len]).to_string()
            };

            Ok(Self {
                name,
                flags: msg.ifm_flags as _,
                in_bytes: msg.ifm_data.ifi_ibytes,
                out_bytes: msg.ifm_data.ifi_obytes,
            })
        }
    }

    #[cfg(test)]
    #[test]
    fn test_monitor() {
        let mon = Monitor::new();
        let mut stats = mon.current().unwrap();
        stats.next().unwrap().unwrap();
    }
}
