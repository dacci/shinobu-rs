#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::*;

#[cfg(not(any(target_os = "macos")))]
compile_error!("Unsupported platform");

#[derive(Debug)]
pub struct StorageStat {
    pub bytes_read: f64,
    pub bytes_written: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_monitor() {
        StorageMonitor::new().unwrap().collect().unwrap();
    }
}
