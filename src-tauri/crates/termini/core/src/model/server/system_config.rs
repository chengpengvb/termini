use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemConfig {
    #[serde(default)]
    pub credit_scale: Option<f64>,

    pub login_url: Option<String>,

    #[serde(default)]
    pub portal_url: Option<String>,

    #[serde(default)]
    pub embedding_dim: Option<i32>,

    #[serde(default)]
    pub website_url: Option<String>,

    #[serde(default)]
    pub vision_max_pixels: Option<i32>,

    #[serde(default)]
    pub tool_mirror_url: Option<String>,

    #[serde(default)]
    pub vision_jpeg_quality: Option<i32>,
    #[serde(default)]
    pub embedding_batch_size: Option<i32>,

    #[serde(default)]
    pub suggest_free_monthly: Option<i32>,

    #[serde(default)]
    pub assist_ai_provide_id: Option<String>,

    #[serde(default)]
    pub suggest_ai_provide_id: Option<String>,

    #[serde(default)]
    pub vision_ai_provide_id: Option<String>,

    #[serde(default)]
    pub image_gen_ai_provide_id: Option<String>,

    #[serde(default)]
    pub embedding_ai_provide_id: Option<String>,
}
