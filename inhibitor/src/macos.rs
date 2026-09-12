use super::InhibitorBuilder;
use objc2_core_foundation::CFString;
use objc2_io_kit::{IOPMAssertionCreateWithName, IOPMAssertionID, IOPMAssertionRelease};
use std::io;

#[derive(Debug, Default)]
pub struct Inhibitor {
    require_display: bool,
}

impl Inhibitor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn builder() -> InhibitorBuilder {
        InhibitorBuilder::default()
    }

    pub fn inhibit(&self) -> io::Result<Assertion> {
        let mut assertion_id = 0;
        let assertion_type = if self.require_display {
            iokit::kIOPMAssertionTypePreventUserIdleDisplaySleep
        } else {
            iokit::kIOPMAssertionTypePreventUserIdleSystemSleep
        };
        let ret = unsafe {
            IOPMAssertionCreateWithName(
                Some(CFString::from_str(assertion_type).as_ref()),
                iokit::kIOPMAssertionLevelOn,
                Some(CFString::from_str("Inhibitor is active").as_ref()),
                &mut assertion_id,
            )
        };
        match ret {
            0 => Ok(Assertion(assertion_id)),
            code => Err(io::Error::from_raw_os_error(code)),
        }
    }

    pub fn require_display(&self) -> bool {
        self.require_display
    }

    pub fn set_require_display(&mut self, require_display: bool) {
        self.require_display = require_display;
    }
}

pub struct Assertion(IOPMAssertionID);

impl Drop for Assertion {
    fn drop(&mut self) {
        IOPMAssertionRelease(self.0);
    }
}

mod iokit {
    #![allow(non_upper_case_globals)]

    pub const kIOPMAssertionTypePreventUserIdleSystemSleep: &str = "PreventUserIdleSystemSleep";
    pub const kIOPMAssertionTypePreventUserIdleDisplaySleep: &str = "PreventUserIdleDisplaySleep";

    pub const kIOPMAssertionLevelOn: u32 = 255;
}
