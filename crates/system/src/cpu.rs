/// Rolling history of CPU usage samples — useful for sparkline rendering.
#[derive(Debug, Clone)]
pub struct CpuHistory {
    pub samples:  std::collections::VecDeque<f32>,
    pub capacity: usize,
}

impl CpuHistory {
    pub fn new(capacity: usize) -> Self {
        Self {
            samples:  std::collections::VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    /// Push a new sample, evicting the oldest if at capacity.
    pub fn push(&mut self, value: f32) {
        if self.samples.len() == self.capacity {
            self.samples.pop_front();
        }
        self.samples.push_back(value);
    }

    /// Average of all samples in the history window.
    pub fn average(&self) -> f32 {
        if self.samples.is_empty() {
            return 0.0;
        }
        self.samples.iter().sum::<f32>() / self.samples.len() as f32
    }
}
