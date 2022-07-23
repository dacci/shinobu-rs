pub struct Historical {
    last: Option<u64>,
    hist: [u64; 900],
    pos: usize,
    len: usize,
}

impl Historical {
    pub fn new() -> Self {
        Self {
            last: None,
            hist: [0; 900],
            pos: 0,
            len: 0,
        }
    }

    pub fn push(&mut self, val: u64) -> Option<u64> {
        if let Some(last) = self.last.replace(val) {
            let diff = val.wrapping_sub(last);
            self.push_diff(diff);
            Some(diff)
        } else {
            None
        }
    }

    pub fn push_diff(&mut self, diff: u64) {
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

    pub fn moving_average(&self, len: usize) -> f64 {
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
