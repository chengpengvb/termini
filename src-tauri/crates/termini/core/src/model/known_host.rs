use termini_macros::base_target;
use serde::{Deserialize, Serialize};

#[base_target(table = "known_host", entity_type = 6)]
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Model {
    pub port: u16,         // SSH 端口
    pub host: String,      // 主机地址（IP / 域名 / IPv6）
    pub sha256: String,    // 公钥 SHA256 指纹（"SHA256:xxx"）
    pub trusted_at: i64,   // 用户选择 ALWAYS 信任的时间（Unix 秒）
    pub algorithm: String, // 公钥算法，例如 "ssh-ed25519" / "ecdsa-sha2-nistp256"
}

pub type KnownHost = Model;
