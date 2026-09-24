use crate::traits::clock::Clock;

#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_utc(&self) -> time::OffsetDateTime {
        time::OffsetDateTime::now_utc()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FrozenClock(pub time::OffsetDateTime);

impl Clock for FrozenClock {
    fn now_utc(&self) -> time::OffsetDateTime {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frozen_clock_stays_put() {
        let t = time::OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap();
        let clock = FrozenClock(t);
        assert_eq!(clock.now_utc(), t);
        assert_eq!(clock.now_utc(), t);
    }
}
