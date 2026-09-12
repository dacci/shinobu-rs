use super::InhibitorBuilder;
use std::io;
use windows_sys::Win32::System::Power::{
    ES_CONTINUOUS, ES_DISPLAY_REQUIRED, ES_SYSTEM_REQUIRED, EXECUTION_STATE,
    SetThreadExecutionState,
};

#[derive(Debug)]
pub struct Inhibitor {
    flags: EXECUTION_STATE,
}

impl Default for Inhibitor {
    fn default() -> Self {
        Self {
            flags: ES_CONTINUOUS | ES_SYSTEM_REQUIRED,
        }
    }
}

impl Inhibitor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn builder() -> InhibitorBuilder {
        InhibitorBuilder::default()
    }

    pub fn inhibit(&self) -> io::Result<Assertion> {
        match unsafe { SetThreadExecutionState(self.flags) } {
            0 => Err(io::Error::last_os_error()),
            _ => Ok(Assertion),
        }
    }

    pub fn require_display(&self) -> bool {
        self.flags & ES_DISPLAY_REQUIRED == ES_DISPLAY_REQUIRED
    }

    pub fn set_require_display(&mut self, require_display: bool) {
        if require_display {
            self.flags |= ES_DISPLAY_REQUIRED;
        } else {
            self.flags &= !ES_DISPLAY_REQUIRED;
        }
    }
}

pub struct Assertion;

impl Drop for Assertion {
    fn drop(&mut self) {
        unsafe { SetThreadExecutionState(ES_CONTINUOUS) };
    }
}
