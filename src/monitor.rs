use crate::sys;
use log::{debug, error, info, warn};
use std::collections::HashMap;
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
    inhibitor: sys::Inhibitor,
    assertion: Option<sys::Assertion>,
    monitor: sys::net::Monitor,
    if_hist: HashMap<String, (Historical, Historical)>,
    hist_in: Historical,
    hist_out: Historical,
    prevent_display_sleep: bool,
}

impl MonitorStat {
    fn new(channel: Receiver<MonitorCommand>) -> Self {
        Self {
            channel,
            inhibitor: sys::Inhibitor::new(),
            assertion: None,
            monitor: sys::net::Monitor::new(),
            if_hist: HashMap::new(),
            hist_in: Historical::new(),
            hist_out: Historical::new(),
            prevent_display_sleep: false,
        }
    }

    fn run(mut self) {
        while let Ok(cmd) = self.channel.recv() {
            match cmd {
                MonitorCommand::Tick => self.tick(),
                MonitorCommand::SetPreventDisplaySleep(keep) => {
                    self.set_prevent_display_sleep(keep)
                }
            }
        }
    }

    fn tick(&mut self) {
        let mut diff_in = 0;
        let mut diff_out = 0;

        let stats = match self.monitor.current() {
            Ok(stats) => stats,
            Err(e) => {
                warn!("Failed to get current status: {e}");
                return;
            }
        };
        for stat in stats {
            let stat = match stat {
                Ok(stat) => stat,
                Err(e) => {
                    warn!("Failed to get interface status: {e}");
                    continue;
                }
            };

            let (hist_in, hist_out) = self
                .if_hist
                .entry(stat.name)
                .or_insert_with_key(|_| (Historical::new(), Historical::new()));
            if let Some(diff) = hist_in.push(stat.in_bytes) {
                diff_in += diff;
            }
            if let Some(diff) = hist_out.push(stat.out_bytes) {
                diff_out += diff;
            }
        }

        self.hist_in.push_diff(diff_in);
        self.hist_out.push_diff(diff_out);

        let medium =
            self.hist_in.moving_average(MA_LENGTH) + self.hist_out.moving_average(MA_LENGTH);
        debug!("{medium:.0} B/s");
        if medium < NET_THRESHOLD && self.assertion.is_some() {
            self.assertion = None;
            info!("Assertion released");
        } else if NET_THRESHOLD <= medium && self.assertion.is_none() {
            match self.inhibitor.inhibit(self.prevent_display_sleep) {
                Ok(assertion) => {
                    self.assertion = Some(assertion);
                    info!("Assertion taken");
                }
                Err(e) => error!("Failed to inhibit: {e}"),
            }
        }
    }

    fn set_prevent_display_sleep(&mut self, prevent: bool) {
        self.prevent_display_sleep = prevent;
        if self.assertion.is_some() {
            match self.inhibitor.inhibit(self.prevent_display_sleep) {
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
    pub fn new() -> Self {
        let (tx, rx) = channel();

        let stat = MonitorStat::new(rx);
        let handle = std::thread::spawn(move || stat.run());

        Self {
            channel: Some(tx),
            handle: Some(handle),
        }
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

struct Historical {
    last: Option<u64>,
    hist: [u64; 900],
    pos: usize,
    len: usize,
}

impl Historical {
    fn new() -> Self {
        Self {
            last: None,
            hist: [0; 900],
            pos: 0,
            len: 0,
        }
    }

    fn push(&mut self, val: u64) -> Option<u64> {
        if let Some(last) = self.last.replace(val) {
            let diff = val.wrapping_sub(last);
            self.push_diff(diff);
            Some(diff)
        } else {
            None
        }
    }

    fn push_diff(&mut self, diff: u64) {
        self.pos = self.pos.wrapping_sub(1).min(self.hist.len() - 1);
        self.hist[self.pos] = diff;
        self.len = (self.len + 1).min(self.hist.len());
    }

    fn take(&self, len: usize) -> impl Iterator<Item = &u64> {
        self.hist[self.pos..]
            .iter()
            .chain(self.hist[..self.pos].iter())
            .take(len.min(self.len))
    }

    fn moving_average(&self, len: usize) -> f64 {
        let (sum, count) = self
            .take(len)
            .fold((0, 0usize), |(sum, count), val| (sum + val, count + 1));
        sum as f64 / count as f64
    }
}

#[cfg(test)]
#[test]
fn test_historical() {
    let mut hist = Historical::new();
    assert!(hist.moving_average(60).is_nan());
    assert_eq!(hist.push(0), None);
    assert!(hist.moving_average(60).is_nan());
    assert_eq!(hist.push(1), Some(1));
    assert_eq!(hist.moving_average(60), 1.0);
    assert_eq!(hist.push(3), Some(2));
    assert_eq!(hist.moving_average(60), 1.5);

    let mut take = hist.take(3);
    assert_eq!(take.next(), Some(&2));
    assert_eq!(take.next(), Some(&1));
    assert_eq!(take.next(), None);
}
