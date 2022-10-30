mod monitor;
mod sys;
mod ui;

use anyhow::Result;

fn main() -> Result<()> {
    env_logger::init();

    let (tx, rx) = tokio::sync::oneshot::channel();
    let monitor = std::thread::spawn(|| monitor::main(rx));

    ui::main();

    if tx.send(()).is_ok() {
        let _ = monitor.join();
    }

    Ok(())
}
