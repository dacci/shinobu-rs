#![windows_subsystem = "windows"]

mod monitor;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "windows")]
mod windows;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    use tracing_subscriber::prelude::*;
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            tracing_subscriber::filter::EnvFilter::builder()
                .with_default_directive(tracing_subscriber::filter::LevelFilter::WARN.into())
                .from_env()
                .expect("failed to initialize tracing"),
        )
        .init();

    cfg_select! {
        target_os = "macos" => macos::main()?,
        target_os = "windows" => windows::main()?,
    }

    Ok(())
}
