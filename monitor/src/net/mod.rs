#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::*;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use windows::*;

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
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
