use super::TrafficStat;
use crate::perf::*;

pub struct TrafficMonitor {
    query: Query,
    in_counters: Vec<Counter>,
    out_counters: Vec<Counter>,
}

impl TrafficMonitor {
    const OBJECT_NAME: &str = "Network Adapter";
    const IN_COUNTER_NAME: &str = "Bytes Received/sec";
    const OUT_COUNTER_NAME: &str = "Bytes Sent/sec";

    pub fn new() -> Result<Self> {
        let query = Query::new()?;

        let in_paths = Counter::make_path(Self::OBJECT_NAME, "*", Self::IN_COUNTER_NAME)
            .and_then(|p| Counter::expand_path(&p))?;
        let out_paths = Counter::make_path(Self::OBJECT_NAME, "*", Self::OUT_COUNTER_NAME)
            .and_then(|p| Counter::expand_path(&p))?;

        let in_counters = in_paths
            .into_iter()
            .map(|p| query.add_counter(&p))
            .filter_map(Result::ok)
            .collect();
        let out_counters = out_paths
            .into_iter()
            .map(|p| query.add_counter(&p))
            .filter_map(Result::ok)
            .collect();

        query.collect_data()?;

        Ok(Self {
            query,
            in_counters,
            out_counters,
        })
    }

    pub fn collect(&self) -> Result<TrafficStat> {
        self.query.collect_data()?;

        Ok(TrafficStat {
            in_bytes: self
                .in_counters
                .iter()
                .map(Counter::format_double)
                .filter_map(Result::ok)
                .sum(),
            out_bytes: self
                .out_counters
                .iter()
                .map(Counter::format_double)
                .filter_map(Result::ok)
                .sum(),
        })
    }
}
