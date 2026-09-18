use crate::perf::{Counter, Query};
use crate::storage::StorageStat;
use std::io::Result;

pub struct StorageMonitor {
    query: Query,
    in_counter: Counter,
    out_counter: Counter,
}

impl StorageMonitor {
    const OBJECT_NAME: &str = "PhysicalDisk";
    const INSTANCE_NAME: &str = "_Total";
    const IN_COUTER_NAME: &str = "Disk Read Bytes/sec";
    const OUT_COUNTER_NAME: &str = "Disk Write Bytes/sec";

    pub fn new() -> Result<Self> {
        let query = Query::new()?;

        let in_path =
            Counter::make_path(Self::OBJECT_NAME, Self::INSTANCE_NAME, Self::IN_COUTER_NAME)?;
        let out_path = Counter::make_path(
            Self::OBJECT_NAME,
            Self::INSTANCE_NAME,
            Self::OUT_COUNTER_NAME,
        )?;

        let in_counter = query.add_counter(&in_path)?;
        let out_counter = query.add_counter(&out_path)?;

        query.collect_data()?;

        Ok(Self {
            query,
            in_counter,
            out_counter,
        })
    }

    pub fn collect(&mut self) -> Result<StorageStat> {
        self.query.collect_data()?;

        Ok(StorageStat {
            bytes_read: self.in_counter.format_double()?,
            bytes_written: self.out_counter.format_double()?,
        })
    }
}
