use time::Duration;
use super::flow::Timestamp;

pub struct Timer {
    delay: Duration,
    next:  Timestamp,
}

impl Timer {
    pub fn new(delay: Duration) -> Self {
        let next = Timestamp::zero();
        Timer{
            delay: delay,
            next:  next,
        }
    }

    pub fn ready(&mut self, ts: Timestamp) -> bool {
        if self.next <= ts {
            let delay = self.delay;
            self.next = ts + delay;
            true
        } else {
            false
        }
    }
}
