use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct RespTree {
    id: String,
    title: String,
    r#type: i16,
    data: Option<String>,
    children: Option<Vec<RespTree>>,
}

impl RespTree {
    pub fn new(id: String, title: String, r#type: i16, data: Option<String>, children: Option<Vec<RespTree>>) -> Self {
        Self { id, title, r#type, data, children }
    }
}
