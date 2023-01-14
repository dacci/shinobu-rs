use crate::sys;
use anyhow::Result;
use futures::StreamExt;
use log::{debug, info, warn};
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::{interval, MissedTickBehavior};

const NET_THRESHOLD: f64 = 50.0 * 1024.0;
const MA_LENGTH: usize = 300;

pub(super) fn main(signal: tokio::sync::oneshot::Receiver<()>) {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            tokio::select! {
                _ = signal => {},
                _ = async_main() => {},
            }
        })
}

async fn async_main() -> Result<()> {
    let inhibitor = sys::Inhibitor::new().await?;
    let mut assertion = None;

    let monitor = sys::net::Monitor::new().await?;

    let mut if_hist = HashMap::new();
    let mut hist_in = Historical::new();
    let mut hist_out = Historical::new();

    let mut interval = interval(Duration::from_secs(1));
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        interval.tick().await;

        let mut diff_in = 0;
        let mut diff_out = 0;

        let stats = match monitor.current().await {
            Ok(stats) => stats,
            Err(e) => {
                warn!("Failed to get current status: {e}");
                continue;
            }
        };
        tokio::pin!(stats);
        while let Some(stat) = stats.next().await {
            let stat = match stat {
                Ok(stat) => stat,
                Err(e) => {
                    warn!("Failed to get interface status: {e}");
                    continue;
                }
            };

            let (hist_in, hist_out) = if_hist
                .entry(stat.name)
                .or_insert_with_key(|_| (Historical::new(), Historical::new()));
            if let Some(diff) = hist_in.push(stat.in_bytes) {
                diff_in += diff;
            }
            if let Some(diff) = hist_out.push(stat.out_bytes) {
                diff_out += diff;
            }
        }

        hist_in.push_diff(diff_in);
        hist_out.push_diff(diff_out);

        let medium = hist_in.moving_average(MA_LENGTH) + hist_out.moving_average(MA_LENGTH);
        debug!("{medium:.0} B/s");
        if medium < NET_THRESHOLD && assertion.is_some() {
            assertion = None;
            info!("Assertion taken");
        } else if NET_THRESHOLD <= medium && assertion.is_none() {
            assertion = Some(inhibitor.inhibit().await);
            info!("Assertion released");
        }
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
