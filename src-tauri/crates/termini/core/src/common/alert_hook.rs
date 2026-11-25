use std::sync::OnceLock;

static ALERT_ENQUEUE: OnceLock<Box<dyn Fn(String) + Send + Sync>> = OnceLock::new();

pub fn register_alert_enqueue(hook: impl Fn(String) + Send + Sync + 'static) {
    if ALERT_ENQUEUE.set(Box::new(hook)).is_err() {
        panic!("alert enqueue hook already registered");
    }
}

pub fn enqueue_alert(alert_id: String) {
    match ALERT_ENQUEUE.get() {
        Some(hook) => hook(alert_id),
        None => eprintln!("[alert] enqueue hook missing, alert_id={}", alert_id),
    }
}
