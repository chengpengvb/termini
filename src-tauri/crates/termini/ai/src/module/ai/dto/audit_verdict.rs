use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditVerdict {
    pub allow: bool,
    pub reason: String,
    pub checked: String,
}

impl AuditVerdict {
    pub fn allow(checked: impl Into<String>) -> Self {
        Self { allow: true, reason: String::new(), checked: checked.into() }
    }

    pub fn reject(reason: impl Into<String>, checked: impl Into<String>) -> Self {
        Self { allow: false, reason: reason.into(), checked: checked.into() }
    }
}
