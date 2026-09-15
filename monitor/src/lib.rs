pub mod net;

pub struct Historical {
    hist: Vec<f64>,
    pos: usize,
    len: usize,
}

impl Default for Historical {
    fn default() -> Self {
        Historical::with_len(900)
    }
}

impl Historical {
    pub fn with_len(len: usize) -> Self {
        Self {
            hist: vec![0.0; len],
            pos: 0,
            len: 0,
        }
    }

    pub fn push(&mut self, diff: f64) {
        self.pos = self.pos.wrapping_sub(1).min(self.hist.len() - 1);
        self.hist[self.pos] = diff;
        self.len = (self.len + 1).min(self.hist.len());
    }

    pub fn moving_average(&self, len: usize) -> f64 {
        let (sum, count) = self
            .take(len)
            .fold((0.0, 0usize), |(sum, count), val| (sum + val, count + 1));
        sum / count as f64
    }

    fn take(&self, len: usize) -> impl Iterator<Item = &f64> {
        self.hist[self.pos..]
            .iter()
            .chain(self.hist[..self.pos].iter())
            .take(len.min(self.len))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_historical() {
        let mut hist = Historical::default();
        assert!(hist.moving_average(60).is_nan());
        hist.push(1.0);
        assert_eq!(hist.moving_average(60), 1.0);
        hist.push(2.0);
        assert_eq!(hist.moving_average(60), 1.5);
        hist.push(3.0);
        assert_eq!(hist.moving_average(60), 2.0);
    }
}
