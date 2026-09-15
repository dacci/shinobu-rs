use super::TrafficStat;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::io::{Error, Result};
use std::ptr::null_mut;
use std::time::Instant;

struct InterfaceStat {
    in_bytes: u64,
    out_bytes: u64,
}

pub struct TrafficMonitor {
    prev_time: Instant,
    prev_stats: HashMap<usize, InterfaceStat>,
}

impl TrafficMonitor {
    pub fn new() -> Result<Self> {
        TrafficStats::collect().map(|stats| Self {
            prev_time: stats.timestamp,
            prev_stats: stats
                .into_iter()
                .map(|msg| {
                    (
                        msg.ifm_index as _,
                        InterfaceStat {
                            in_bytes: msg.ifm_data.ifi_ibytes,
                            out_bytes: msg.ifm_data.ifi_obytes,
                        },
                    )
                })
                .collect(),
        })
    }

    pub fn collect(&mut self) -> Result<TrafficStat> {
        let stats = TrafficStats::collect()?;

        let mut in_bytes = 0.0;
        let mut out_bytes = 0.0;
        for msg in &stats {
            match self.prev_stats.entry(msg.ifm_index as _) {
                Entry::Occupied(mut e) => {
                    let stat = e.get_mut();

                    in_bytes += msg.ifm_data.ifi_ibytes.wrapping_sub(stat.in_bytes) as f64;
                    stat.in_bytes = msg.ifm_data.ifi_ibytes;

                    out_bytes += msg.ifm_data.ifi_obytes.wrapping_sub(stat.out_bytes) as f64;
                    stat.out_bytes = msg.ifm_data.ifi_obytes;
                }
                Entry::Vacant(e) => {
                    e.insert(InterfaceStat {
                        in_bytes: msg.ifm_data.ifi_ibytes,
                        out_bytes: msg.ifm_data.ifi_obytes,
                    });
                }
            }
        }

        let interval = stats.timestamp.duration_since(self.prev_time).as_secs_f64();
        self.prev_time = stats.timestamp;

        Ok(TrafficStat {
            in_bytes: in_bytes / interval,
            out_bytes: out_bytes / interval,
        })
    }
}

pub struct TrafficStats {
    timestamp: Instant,
    buf: Vec<u8>,
}

impl TrafficStats {
    pub fn collect() -> Result<Self> {
        let mut name = [libc::CTL_NET, libc::PF_ROUTE, 0, 0, libc::NET_RT_IFLIST2, 0];
        let mut len = 0;
        let res = unsafe {
            libc::sysctl(
                name.as_mut_ptr(),
                name.len() as _,
                null_mut(),
                &mut len,
                null_mut(),
                0,
            )
        };
        if res != 0 {
            return Err(Error::last_os_error());
        }

        let mut buf = vec![0; len];
        let res = unsafe {
            libc::sysctl(
                name.as_mut_ptr(),
                name.len() as _,
                buf.as_mut_ptr() as _,
                &mut len,
                null_mut(),
                0,
            )
        };
        if res != 0 {
            return Err(Error::last_os_error());
        }

        Ok(Self {
            timestamp: Instant::now(),
            buf,
        })
    }
}

impl<'a> IntoIterator for &'a TrafficStats {
    type Item = <InterfaceIter<'a> as Iterator>::Item;
    type IntoIter = InterfaceIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        InterfaceIter {
            buf: &self.buf,
            pos: 0,
        }
    }
}

pub struct InterfaceIter<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Iterator for InterfaceIter<'a> {
    type Item = &'a libc::if_msghdr2;

    fn next(&mut self) -> Option<Self::Item> {
        while self.pos < self.buf.len() {
            let msg = unsafe {
                self.buf
                    .as_ptr()
                    .add(self.pos)
                    .cast::<libc::if_msghdr2>()
                    .as_ref_unchecked()
            };

            if msg.ifm_msglen == 0 {
                break;
            }
            self.pos += msg.ifm_msglen as usize;

            if msg.ifm_type == libc::RTM_IFINFO2 as _ {
                return Some(msg);
            }
        }

        None
    }
}
