pub(crate) use std::io::{Error, Result};
use std::ptr::{null, null_mut};
use windows_sys::Win32::Foundation::ERROR_SUCCESS;
use windows_sys::Win32::System::Performance::*;

const PDH_FMT_NOCAP100: PDH_FMT = 0x00008000;

#[repr(transparent)]
pub(crate) struct Query(PDH_HQUERY);

// SAFETY: PDH_HQUERY is an opaque value, not a pointer
unsafe impl Send for Query {}

impl Drop for Query {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { PdhCloseQuery(self.0) };
        }
    }
}

impl Query {
    pub fn new() -> Result<Self> {
        let mut handle = null_mut();
        match unsafe { PdhOpenQueryW(null(), 0, &mut handle) } {
            ERROR_SUCCESS => Ok(Self(handle)),
            code => Err(Error::from_raw_os_error(code as _)),
        }
    }

    pub fn add_counter(&self, path: &[u16]) -> Result<Counter> {
        let mut handle = null_mut();
        match unsafe { PdhAddCounterW(self.0, path.as_ptr(), 0, &mut handle) } {
            ERROR_SUCCESS => Ok(Counter(handle)),
            code => Err(Error::from_raw_os_error(code as _)),
        }
    }

    pub fn collect_data(&self) -> Result<()> {
        match unsafe { PdhCollectQueryData(self.0) } {
            ERROR_SUCCESS => Ok(()),
            code => Err(Error::from_raw_os_error(code as _)),
        }
    }
}

#[repr(transparent)]
pub(crate) struct Counter(PDH_HCOUNTER);

// SAFETY: PDH_HCOUNTER is an opaque value, not a pointer
unsafe impl Send for Counter {}

impl Drop for Counter {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { PdhRemoveCounter(self.0) };
        }
    }
}

impl Counter {
    pub fn make_path(object: &str, instance: &str, counter: &str) -> Result<Vec<u16>> {
        let mut object = object.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
        let mut instance = instance.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
        let mut counter = counter.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
        let elements = PDH_COUNTER_PATH_ELEMENTS_W {
            szMachineName: null_mut(),
            szObjectName: object.as_mut_ptr(),
            szInstanceName: instance.as_mut_ptr(),
            szParentInstance: null_mut(),
            dwInstanceIndex: 0,
            szCounterName: counter.as_mut_ptr(),
        };

        let mut path = [0; PDH_MAX_COUNTER_PATH as _];
        let mut len = PDH_MAX_COUNTER_PATH;
        match unsafe { PdhMakeCounterPathW(&elements, path.as_mut_ptr(), &mut len, 0) } {
            ERROR_SUCCESS => Ok(Vec::from(&path[..len as _])),
            code => Err(Error::from_raw_os_error(code as _)),
        }
    }

    pub fn expand_path(path: &[u16]) -> Result<Vec<Vec<u16>>> {
        let mut len = 0;
        let code =
            unsafe { PdhExpandWildCardPathW(null(), path.as_ptr(), null_mut(), &mut len, 0) };
        if code != PDH_MORE_DATA {
            return Err(Error::from_raw_os_error(code as _));
        }

        let mut buf = vec![0; len as _];
        let code =
            unsafe { PdhExpandWildCardPathW(null(), path.as_ptr(), buf.as_mut_ptr(), &mut len, 0) };
        if code != ERROR_SUCCESS {
            return Err(Error::from_raw_os_error(code as _));
        }

        Ok(buf
            .split(|c| *c == 0)
            .filter(|s| !s.is_empty())
            .map(Vec::from)
            .collect())
    }

    fn format_value(&self, format: PDH_FMT) -> Result<PDH_FMT_COUNTERVALUE> {
        let mut value = PDH_FMT_COUNTERVALUE::default();
        match unsafe { PdhGetFormattedCounterValue(self.0, format, null_mut(), &mut value) } {
            ERROR_SUCCESS => Ok(value),
            code => Err(Error::from_raw_os_error(code as _)),
        }
    }

    pub fn format_double(&self) -> Result<f64> {
        self.format_value(PDH_FMT_DOUBLE | PDH_FMT_NOCAP100)
            .map(|v| unsafe { v.Anonymous.doubleValue })
    }
}
