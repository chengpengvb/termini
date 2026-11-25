use async_trait::async_trait;

pub struct AppContext {}

#[async_trait]
pub trait BaseApp: Send + Sync {
    fn phase(&self) -> i16 {
        200
    }

    fn is_core(&self) -> bool {
        true
    }

    fn name(&self) -> &'static str {
        "unknown"
    }

    async fn start(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        Ok(())
    }

    async fn on_exit(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        Ok(())
    }
}
