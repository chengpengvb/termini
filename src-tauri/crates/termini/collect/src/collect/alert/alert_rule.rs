use crate::model::job_indicator::JobIndicatorAlertRuleOp;

pub fn is_hit(op: JobIndicatorAlertRuleOp, value: &str, threshold: &str) -> bool {
    let value = value.trim();
    let threshold = threshold.trim();
    if let (Ok(value), Ok(threshold)) = (value.parse::<f64>(), threshold.parse::<f64>()) {
        return match op {
            JobIndicatorAlertRuleOp::Gt => value > threshold,
            JobIndicatorAlertRuleOp::Gte => value >= threshold,
            JobIndicatorAlertRuleOp::Lt => value < threshold,
            JobIndicatorAlertRuleOp::Lte => value <= threshold,
            JobIndicatorAlertRuleOp::Eq => (value - threshold).abs() < f64::EPSILON,
        };
    }
    matches!(op, JobIndicatorAlertRuleOp::Eq) && value == threshold
}
