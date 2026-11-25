#[cfg(target_os = "windows")]
use tauri::Manager;
#[cfg(target_os = "windows")]
use anyhow::Context;
use tauri::AppHandle;
use summer::plugin::Service;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use crate::model::resp::resp_model::ResultCode;
use crate::service::service_base_api::BaseApiService;
use store_core::{StoreProduct, StorePurchase, StoreResult};
use crate::service::service_account_token::AccountTokenService;

#[base_app(sort = 1001)]
#[derive(Clone, Service)]
pub struct StoreService {
    #[inject(component)]
    handle: AppHandle,

    #[inject(component)]
    account_token_service: AccountTokenService,
}

#[async_trait]
impl BaseApp for StoreService {
    fn name(&self) -> &'static str {
        "StoreService"
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> Result<()> {
        let svc = self.clone();
        tokio::spawn(async move {
            svc.settle_unfinished().await;
            let mut updates = store_apple::updates();
            while let Some(update) = updates.recv().await {
                match update {
                    Ok(purchase) => svc.settle(purchase).await,
                    Err(error) => summer::tracing::warn!("[store] 交易更新解析失败: {}", error),
                }
            }
        });
        Ok(())
    }
}

impl BaseApiService for StoreService {}

impl StoreService {
    pub async fn list_products(&self, product_ids: Vec<String>) -> Result<Vec<StoreProduct>> {
        platform_list_products(product_ids).await.map_err(|error| anyhow!(error.to_string()))
    }

    pub async fn purchase(&self, product_id: String) -> Result<StorePurchase> {
        let hwnd = self.main_window_hwnd()?;
        platform_purchase(hwnd, product_id).await.map_err(|error| anyhow!(error.to_string()))
    }

    pub async fn restore(&self) -> Result<()> {
        let purchases = platform_restore().await.map_err(|error| anyhow!(error.to_string()))?;
        for mut purchase in purchases {
            self.attach_store_user_key(&mut purchase).await?;
            self.bind_purchase(&purchase).await?;
            platform_finish(&purchase).await.map_err(|error| anyhow!(error.to_string()))?;
        }
        Ok(())
    }

    pub async fn subscribe(&self, product_id: String) -> Result<()> {
        let mut purchase = self.purchase(product_id).await?;
        self.attach_store_user_key(&mut purchase).await?;
        self.bind_purchase(&purchase).await?;
        platform_finish(&purchase).await.map_err(|error| anyhow!(error.to_string()))
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    async fn settle_unfinished(&self) {
        match store_apple::unfinished().await {
            Ok(purchases) => {
                for purchase in purchases {
                    self.settle(purchase).await;
                }
            }
            Err(error) => summer::tracing::warn!("[store] 读取未完成交易失败: {}", error),
        }
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    async fn settle(&self, purchase: StorePurchase) {
        if let Err(error) = self.bind_purchase(&purchase).await {
            summer::tracing::warn!("[store] 交易绑定失败, 留待下次: originalTransactionId={}, {}", purchase.transaction_id, error);
            return;
        }
        if let Err(error) = platform_finish(&purchase).await {
            summer::tracing::warn!("[store] 交易完成失败: originalTransactionId={}, {}", purchase.transaction_id, error);
        }
    }

    /// 微软的订阅号(recurrenceId)在 WinRT 里取不到, 只能交出 Store ID key 让服务端反查。
    #[cfg(target_os = "windows")]
    async fn attach_store_user_key(&self, purchase: &mut StorePurchase) -> Result<()> {
        let ticket = self.service_ticket().await?;
        let profile = self.account_token_service.profile().await?;
        let user_id = profile.id.context("profile id is empty")?;
        purchase.receipt = store_microsoft::purchase_id(ticket, user_id).await.map_err(|error| anyhow!(error.to_string()))?;
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    async fn attach_store_user_key(&self, _purchase: &mut StorePurchase) -> Result<()> {
        Ok(())
    }

    #[cfg(target_os = "windows")]
    async fn service_ticket(&self) -> Result<String> {
        if self.account_token_service.get_token().await?.is_none() {
            return Err(ResultCode::Unauthorized.into());
        }
        Self::api_data(Self::api_post("/api/storeApi/serviceTicket")).await
    }

    async fn bind_purchase(&self, purchase: &StorePurchase) -> Result<()> {
        if self.account_token_service.get_token().await?.is_none() {
            return Err(ResultCode::Unauthorized.into());
        }
        let body = serde_json::json!({
            "store": purchase.store,
            "productId": purchase.product_id,
            "subscriptionId": purchase.transaction_id,
            "receipt": purchase.receipt,
        });
        Self::api_result::<serde_json::Value>(Self::api_post("/api/storeApi/bind").json(&body)).await?;
        Ok(())
    }

    #[cfg(target_os = "windows")]
    fn main_window_hwnd(&self) -> Result<isize> {
        let window = self.handle.get_webview_window("main").ok_or_else(|| anyhow::Error::from(ResultCode::MainWindowNotFound))?;
        let hwnd = window.hwnd().map_err(|error| anyhow!("failed to get window handle: {}", error))?;
        Ok(hwnd.0 as isize)
    }

    #[cfg(not(target_os = "windows"))]
    fn main_window_hwnd(&self) -> Result<isize> {
        let _ = &self.handle;
        Ok(0)
    }
}

#[cfg(target_os = "windows")]
async fn platform_list_products(product_ids: Vec<String>) -> StoreResult<Vec<StoreProduct>> {
    store_microsoft::list_products(product_ids).await
}

#[cfg(target_os = "windows")]
async fn platform_purchase(hwnd: isize, product_id: String) -> StoreResult<StorePurchase> {
    store_microsoft::purchase(hwnd, product_id).await
}

#[cfg(target_os = "windows")]
async fn platform_restore() -> StoreResult<Vec<StorePurchase>> {
    store_microsoft::restore().await
}

#[cfg(target_os = "windows")]
async fn platform_finish(_purchase: &StorePurchase) -> StoreResult<()> {
    Ok(())
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
async fn platform_list_products(product_ids: Vec<String>) -> StoreResult<Vec<StoreProduct>> {
    store_apple::list_products(product_ids).await
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
async fn platform_purchase(hwnd: isize, product_id: String) -> StoreResult<StorePurchase> {
    store_apple::purchase(hwnd, product_id).await
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
async fn platform_restore() -> StoreResult<Vec<StorePurchase>> {
    store_apple::restore().await
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
async fn platform_finish(purchase: &StorePurchase) -> StoreResult<()> {
    store_apple::finish(purchase.transaction_id.clone()).await
}

#[cfg(target_os = "android")]
async fn platform_list_products(product_ids: Vec<String>) -> StoreResult<Vec<StoreProduct>> {
    store_google::list_products(product_ids).await
}

#[cfg(target_os = "android")]
async fn platform_purchase(hwnd: isize, product_id: String) -> StoreResult<StorePurchase> {
    store_google::purchase(hwnd, product_id).await
}

#[cfg(target_os = "android")]
async fn platform_restore() -> StoreResult<Vec<StorePurchase>> {
    store_google::restore().await
}

#[cfg(target_os = "android")]
async fn platform_finish(purchase: &StorePurchase) -> StoreResult<()> {
    store_google::finish(purchase.transaction_id.clone()).await
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "ios", target_os = "android")))]
async fn platform_list_products(_product_ids: Vec<String>) -> StoreResult<Vec<StoreProduct>> {
    Err(store_core::StoreError::Unsupported("linux"))
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "ios", target_os = "android")))]
async fn platform_purchase(_hwnd: isize, _product_id: String) -> StoreResult<StorePurchase> {
    Err(store_core::StoreError::Unsupported("linux"))
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "ios", target_os = "android")))]
async fn platform_restore() -> StoreResult<Vec<StorePurchase>> {
    Err(store_core::StoreError::Unsupported("linux"))
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "ios", target_os = "android")))]
async fn platform_finish(_purchase: &StorePurchase) -> StoreResult<()> {
    Err(store_core::StoreError::Unsupported("linux"))
}
