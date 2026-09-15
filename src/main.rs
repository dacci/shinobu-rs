#![windows_subsystem = "windows"]

mod monitor;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "windows")]
mod windows;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    cfg_select! {
        target_os = "macos" => macos::main()?,
        target_os = "windows" => windows::main()?,
    }

    Ok(())
}
