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

#[derive(Debug, Default)]
pub struct InhibitorBuilder(Inhibitor);

impl InhibitorBuilder {
    pub fn build(self) -> Inhibitor {
        self.0
    }

    pub fn require_display(&mut self, require_display: bool) -> &mut Self {
        self.0.set_require_display(require_display);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inhibit() {
        let inhibitor = Inhibitor::new();
        assert!(inhibitor.inhibit().is_ok());
    }

    #[test]
    fn test_inhibit_require_display() {
        let mut inhibitor = Inhibitor::new();
        inhibitor.set_require_display(true);
        assert!(inhibitor.inhibit().is_ok());
    }
}
