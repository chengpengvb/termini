use std::time::{SystemTime, UNIX_EPOCH};

pub fn now_millis() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).expect("system clock before UNIX_EPOCH").as_millis() as i64
}
