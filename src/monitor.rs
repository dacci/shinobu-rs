use inhibitor::{Assertion, Inhibitor};
use log::{debug, error, info, warn};
use monitor::{Historical, net::TrafficMonitor};
use std::io::Result;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread::JoinHandle;

const NET_THRESHOLD: f64 = 50.0 * 1024.0;
const MA_LENGTH: usize = 300;

enum MonitorCommand {
    Tick,
    SetPreventDisplaySleep(bool),
}

struct MonitorStat {
    channel: Receiver<MonitorCommand>,
    inhibitor: Inhibitor,
    assertion: Option<Assertion>,
    monitor: TrafficMonitor,
    hist_in: Historical,
    hist_out: Historical,
}

impl MonitorStat {
    fn new(channel: Receiver<MonitorCommand>) -> Result<Self> {
        let monitor = TrafficMonitor::new()?;
        Ok(Self {
            channel,
            inhibitor: Inhibitor::new(),
            assertion: None,
            monitor,
            hist_in: Historical::default(),
            hist_out: Historical::default(),
        })
    }

    fn run(mut self) {
        while let Ok(cmd) = self.channel.recv() {
            match cmd {
                MonitorCommand::Tick => self.tick(),
                MonitorCommand::SetPreventDisplaySleep(prevent) => {
                    self.set_prevent_display_sleep(prevent)
                }
            }
        }
    }

    fn tick(&mut self) {
        match self.monitor.collect() {
            Ok(stats) => {
                self.hist_in.push(stats.in_bytes);
                self.hist_out.push(stats.out_bytes);
            }
            Err(e) => {
                warn!("Failed to get current status: {e}");
                return;
            }
        };

        let medium =
            self.hist_in.moving_average(MA_LENGTH) + self.hist_out.moving_average(MA_LENGTH);
        debug!("{medium:.0} B/s");
        if medium < NET_THRESHOLD && self.assertion.is_some() {
            self.assertion = None;
            info!("Assertion released");
        } else if NET_THRESHOLD <= medium && self.assertion.is_none() {
            match self.inhibitor.inhibit() {
                Ok(assertion) => {
                    self.assertion = Some(assertion);
                    info!("Assertion taken");
                }
                Err(e) => error!("Failed to inhibit: {e}"),
            }
        }
    }

    fn set_prevent_display_sleep(&mut self, prevent: bool) {
        self.inhibitor.set_require_display(prevent);
        if self.assertion.is_some() {
            match self.inhibitor.inhibit() {
                Ok(assertion) => {
                    self.assertion.replace(assertion);
                }
                Err(e) => error!("Failed to inhibit: {e}"),
            }
        }
    }
}

pub struct Monitor {
    channel: Option<Sender<MonitorCommand>>,
    handle: Option<JoinHandle<()>>,
}

impl Monitor {
    pub fn new() -> Result<Self> {
        let (tx, rx) = channel();

        let stat = MonitorStat::new(rx)?;
        let handle = std::thread::spawn(move || stat.run());

        Ok(Self {
            channel: Some(tx),
            handle: Some(handle),
        })
    }

    pub fn tick(&self) {
        self.channel
            .as_ref()
            .unwrap()
            .send(MonitorCommand::Tick)
            .unwrap();
    }

    pub fn set_prevent_display_sleep(&self, keep: bool) {
        self.channel
            .as_ref()
            .unwrap()
            .send(MonitorCommand::SetPreventDisplaySleep(keep))
            .unwrap();
    }
}

impl Drop for Monitor {
    fn drop(&mut self) {
        self.channel.take().unwrap();
        self.handle.take().unwrap().join().unwrap();
    }
}
