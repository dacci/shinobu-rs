#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::*;

#[cfg(not(any(target_os = "macos")))]
compile_error!("Unsupported platform");

#[derive(Debug)]
pub struct TrafficStat {
    pub in_bytes: f64,
    pub out_bytes: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_traffic_monitor() {
        TrafficMonitor::new().unwrap().collect().unwrap();
    }
}
