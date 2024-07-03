use std::time::{Duration, SystemTime, UNIX_EPOCH};
/// Computes the amount of time that has passed between the timestamp, and now.
/// Returns 0 if now is before the timestamp.
pub(crate) fn duration_since_epoch_timestamp(timestamp: u64, now: SystemTime) -> Duration {
    let start = UNIX_EPOCH + Duration::from_secs(timestamp);
    now.duration_since(start).unwrap_or_else(|_| Duration::new(0, 0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn test_duration_since_epoch_timestamp_future() {
        let now = SystemTime::now();
        let future_timestamp = now.duration_since(UNIX_EPOCH).unwrap().as_secs() + 100;
        assert_eq!(duration_since_epoch_timestamp(future_timestamp, now).as_secs(), 0);
    }

    #[test]
    fn test_duration_since_epoch_timestamp_past() {
        let now = SystemTime::now();
        let past_timestamp = now.duration_since(UNIX_EPOCH).unwrap().as_secs() - 100;
        assert_eq!(duration_since_epoch_timestamp(past_timestamp, now).as_secs(), 100);
    }

    #[test]
    fn test_duration_since_epoch_timestamp_exact_now() {
        let now = SystemTime::now();
        let exact_now_timestamp = now.duration_since(UNIX_EPOCH).unwrap().as_secs();
        assert_eq!(duration_since_epoch_timestamp(exact_now_timestamp, now).as_secs(), 0);
    }

    #[test]
    fn test_duration_since_epoch_timestamp_epoch() {
        let now = SystemTime::now();
        let epoch_timestamp = 0; // This is the UNIX_EPOCH timestamp
        let expected_duration = now.duration_since(UNIX_EPOCH).unwrap().as_secs();
        assert_eq!(
            duration_since_epoch_timestamp(epoch_timestamp, now).as_secs(),
            expected_duration
        );
    }
}
