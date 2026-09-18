use crate::io_kit::{
    io_service_get_matching_service, io_service_matching, kIOBlockStorageDriver,
    kIOBlockStorageDriverStatisticsBytesReadKey, kIOBlockStorageDriverStatisticsBytesWrittenKey,
    kIOBlockStorageDriverStatisticsKey,
};
use crate::storage::StorageStat;
use objc2_core_foundation::{CFDictionary, CFNumber, CFRetained, CFString, kCFAllocatorDefault};
use std::io::{Error, ErrorKind, Result};
use std::time::Instant;

pub struct StorageMonitor {
    service_match: CFRetained<CFDictionary>,
    prev_time: Instant,
    prev_bytes_read: i64,
    prev_bytes_written: i64,
}

impl StorageMonitor {
    pub fn new() -> Result<Self> {
        let service_match = io_service_matching(kIOBlockStorageDriver)
            .ok_or_else(|| Error::from(ErrorKind::NotFound))?
            .downcast::<CFDictionary>()
            .unwrap();

        let (read, written) = Self::collect_raw(service_match.clone())?;
        let prev_time = Instant::now();

        Ok(Self {
            service_match,
            prev_time,
            prev_bytes_read: read,
            prev_bytes_written: written,
        })
    }

    fn collect_raw(matches: CFRetained<CFDictionary>) -> Result<(i64, i64)> {
        let Some(service) = io_service_get_matching_service(0, Some(matches)) else {
            return Err(Error::from(ErrorKind::NotFound));
        };

        let Some(stats) = service.io_registry_entry_create_cf_property(
            Some(CFString::from_static_str(kIOBlockStorageDriverStatisticsKey).as_ref()),
            unsafe { kCFAllocatorDefault },
            0,
        ) else {
            return Err(Error::from(ErrorKind::InvalidInput));
        };

        let stats = stats.downcast::<CFDictionary>().unwrap();

        let stats = unsafe { stats.cast_unchecked::<CFString, CFNumber>() };
        let bytes_read = stats
            .get(CFString::from_static_str(kIOBlockStorageDriverStatisticsBytesReadKey).as_ref())
            .ok_or_else(|| Error::from(ErrorKind::InvalidData))?
            .as_i64()
            .unwrap();
        let bytes_written = stats
            .get(CFString::from_static_str(kIOBlockStorageDriverStatisticsBytesWrittenKey).as_ref())
            .ok_or_else(|| Error::from(ErrorKind::InvalidData))?
            .as_i64()
            .unwrap();

        Ok((bytes_read, bytes_written))
    }

    pub fn collect(&mut self) -> Result<StorageStat> {
        let (bytes_read, bytes_written) = Self::collect_raw(self.service_match.clone())?;
        let timestamp = Instant::now();

        let interval = timestamp.duration_since(self.prev_time).as_secs_f64();
        self.prev_time = timestamp;

        let diff_read = bytes_read.wrapping_sub(self.prev_bytes_read) as f64;
        self.prev_bytes_read = bytes_read;

        let diff_written = bytes_written.wrapping_sub(self.prev_bytes_written) as f64;
        self.prev_bytes_written = bytes_written;

        Ok(StorageStat {
            bytes_read: diff_read / interval,
            bytes_written: diff_written / interval,
        })
    }
}
