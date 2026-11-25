use summer_web::axum::Json;
use summer::plugin::Service;
use async_trait::async_trait;
use crate::base::base_app::BaseApp;
use store_core::{StoreProduct, StorePurchase};
use termini_macros::{base_app, control_routes};
use crate::model::resp::resp_model::ResultModel;
use crate::service::service_store::StoreService;
use crate::model::req::req_store::{ReqStoreProducts, ReqStorePurchase};

#[base_app(sort = 421)]
#[derive(Clone, Service)]
pub struct StoreController {
    #[inject(component)]
    store_service: StoreService,
}

#[async_trait]
impl BaseApp for StoreController {
    fn phase(&self) -> i16 {
        40
    }

    fn name(&self) -> &'static str {
        "StoreController"
    }
}

#[control_routes(prefix = "/store")]
impl StoreController {
    #[summer_web::post("/products")]
    async fn products(&self, Json(body): Json<ReqStoreProducts>) -> ResultModel<Vec<StoreProduct>> {
        ResultModel::from_result(self.store_service.list_products(body.product_ids).await)
    }

    #[summer_web::post("/purchase")]
    async fn purchase(&self, Json(body): Json<ReqStorePurchase>) -> ResultModel<StorePurchase> {
        ResultModel::from_result(self.store_service.purchase(body.product_id).await)
    }

    #[summer_web::post("/restore")]
    async fn restore(&self) -> ResultModel<()> {
        ResultModel::from_result(self.store_service.restore().await)
    }

    #[summer_web::post("/subscribe")]
    async fn subscribe(&self, Json(body): Json<ReqStorePurchase>) -> ResultModel<()> {
        ResultModel::from_result(self.store_service.subscribe(body.product_id).await)
    }
}
