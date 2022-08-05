use anyhow::{anyhow, Result};
use core_foundation::base::TCFType;
use core_foundation::string::CFString;

pub struct Inhibitor;

impl Inhibitor {
    pub async fn new() -> Result<Self> {
        Ok(Self)
    }

    pub async fn inhibit(&self) -> Result<Assertion> {
        let mut assertion_id = 0;
        let ret = unsafe {
            iokit::IOPMAssertionCreateWithDescription(
                CFString::from_static_string(iokit::kIOPMAssertionTypePreventUserIdleDisplaySleep)
                    .as_concrete_TypeRef(),
                CFString::from_static_string("Shinobu").as_concrete_TypeRef(),
                CFString::from_static_string("System activity").as_concrete_TypeRef(),
                CFString::from_static_string("Shinobu is preventing sleep.").as_concrete_TypeRef(),
                CFString::from_static_string(iokit::kLocalizationBundlePath).as_concrete_TypeRef(),
                0.0,
                CFString::from_static_string(iokit::kIOPMAssertionTimeoutActionRelease)
                    .as_concrete_TypeRef(),
                &mut assertion_id,
            )
        };
        match ret {
            iokit::kIOReturnSuccess => Ok(Assertion(assertion_id)),
            _ => Err(anyhow!(
                "IOPMAssertionCreateWithDescription() failed: {ret}"
            )),
        }
    }
}

pub struct Assertion(iokit::IOPMAssertionID);

impl Drop for Assertion {
    fn drop(&mut self) {
        unsafe { iokit::IOPMAssertionRelease(self.0) };
    }
}

mod iokit {
    #![allow(unused, non_upper_case_globals)]

    use core_foundation_sys::date::CFTimeInterval;
    use core_foundation_sys::string::CFStringRef;

    pub type IOReturn = i32;
    pub type IOPMAssertionID = u32;

    pub const kIOReturnSuccess: IOReturn = 0;

    pub const kIOPMAssertionTypePreventUserIdleSystemSleep: &str = "PreventUserIdleSystemSleep";
    pub const kIOPMAssertionTypePreventUserIdleDisplaySleep: &str = "PreventUserIdleDisplaySleep";
    pub const kIOPMAssertionTypePreventSystemSleep: &str = "PreventSystemSleep";
    pub const kIOPMAssertionUserIsActive: &str = "UserIsActive";
    pub const kIOPMAssertPreventDiskIdle: &str = "PreventDiskIdle";

    pub const kLocalizationBundlePath: &str = "/System/Library/CoreServices/powerd.bundle";

    pub const kIOPMAssertionTimeoutActionLog: &str = "TimeoutActionLog";
    pub const kIOPMAssertionTimeoutActionTurnOff: &str = "TimeoutActionTurnOff";
    pub const kIOPMAssertionTimeoutActionRelease: &str = "TimeoutActionRelease";

    #[link(name = "IOKit", kind = "framework")]
    extern "C" {
        pub fn IOPMAssertionCreateWithDescription(
            AssertionType: CFStringRef,
            Name: CFStringRef,
            Details: CFStringRef,
            HumanReadableReason: CFStringRef,
            LocalizationBundlePath: CFStringRef,
            Timeout: CFTimeInterval,
            TimeoutAction: CFStringRef,
            AssertionID: *mut IOPMAssertionID,
        ) -> IOReturn;

        pub fn IOPMAssertionRelease(AssertionID: IOPMAssertionID) -> IOReturn;
    }
}

pub mod net {
    use futures::{stream, Stream, StreamExt};
    use std::io::{Error, Result};
    use std::marker::PhantomData;
    use std::ptr::null_mut;

    pub struct Monitor;

    impl Monitor {
        pub async fn new() -> Result<Self> {
            Ok(Self)
        }

        pub async fn current(&self) -> Result<impl Stream<Item = Result<Stat>>> {
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

            Ok(stream::iter(Routes {
                buf,
                pos: 0,
                _a: PhantomData::default(),
            })
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
    #[tokio::test]
    async fn test_monitor() {
        let mon = Monitor::new().await.unwrap();
        let mut stats = mon.current().await.unwrap();
        stats.next().await.unwrap().unwrap();
    }
}
