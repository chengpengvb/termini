use serde::Deserialize;

pub const CODE_OK: i32 = 200;
pub const CODE_UNAUTHORIZED: i32 = 401;
pub const CODE_UPGRADE_REQUIRED: i32 = 426;

#[derive(Debug, Deserialize)]
pub struct ResultModel<T> {
    pub code: i32,
    pub message: String,
    pub data: Option<T>,
}
