use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SshIdentityMaps {
    pub uid_to_name: HashMap<u32, String>,
    pub gid_to_name: HashMap<u32, String>,
}

impl SshIdentityMaps {
    pub fn is_empty(&self) -> bool {
        self.uid_to_name.is_empty() && self.gid_to_name.is_empty()
    }
}
