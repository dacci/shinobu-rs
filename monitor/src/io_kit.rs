#![allow(non_upper_case_globals)]

use libc::mach_port_t;
use objc2_core_foundation::{
    CFAllocator, CFDictionary, CFMutableDictionary, CFRetained, CFString, CFType,
};
use objc2_io_kit::{
    IOObjectRelease, IOOptionBits, IORegistryEntryCreateCFProperty, IOServiceGetMatchingService,
    IOServiceMatching, io_object_t,
};
use std::ffi::CStr;

pub(crate) const kIOBlockStorageDriver: &CStr = c"IOBlockStorageDriver";

pub(crate) const kIOBlockStorageDriverStatisticsKey: &str = "Statistics";
pub(crate) const kIOBlockStorageDriverStatisticsBytesReadKey: &str = "Bytes (Read)";
pub(crate) const kIOBlockStorageDriverStatisticsBytesWrittenKey: &str = "Bytes (Write)";

#[repr(transparent)]
pub(crate) struct IOObject(io_object_t);

impl Drop for IOObject {
    fn drop(&mut self) {
        if self.0 != 0 {
            IOObjectRelease(self.0);
        }
    }
}

impl IOObject {
    pub fn io_registry_entry_create_cf_property(
        &self,
        key: Option<&CFString>,
        allocator: Option<&CFAllocator>,
        options: IOOptionBits,
    ) -> Option<CFRetained<CFType>> {
        unsafe { IORegistryEntryCreateCFProperty(self.0, key, allocator, options) }
    }
}

pub(crate) fn io_service_matching(name: &CStr) -> Option<CFRetained<CFMutableDictionary>> {
    unsafe { IOServiceMatching(name.as_ptr()) }
}

pub(crate) fn io_service_get_matching_service(
    main_port: mach_port_t,
    matching: Option<CFRetained<CFDictionary>>,
) -> Option<IOObject> {
    match unsafe { IOServiceGetMatchingService(main_port, matching) } {
        0 => None,
        handle => Some(IOObject(handle)),
    }
}
