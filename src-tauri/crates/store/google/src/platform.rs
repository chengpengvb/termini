use jni::JavaVM;
use jni::JNIEnv;
use jni::errors::Error as JniError;
use jni::objects::{JClass, JObject, JValue};
use crate::txn::{store_purchase_from_txn, GoogleTxn};
use store_core::{StoreError, StoreProduct, StorePurchase, StoreResult};

const BRIDGE_CLASS: &str = "app.termini.store.PlayBillingBridge";

pub async fn list_products(product_ids: Vec<String>) -> StoreResult<Vec<StoreProduct>> {
    spawn_blocking(move || serde_products(&call_bridge("listProducts", product_ids.join(","))?)).await
}

pub async fn purchase(_hwnd: isize, product_id: String) -> StoreResult<StorePurchase> {
    spawn_blocking(move || {
        let token = call_bridge("purchase", product_id.clone())?;
        Ok(store_purchase_from_txn(GoogleTxn { product_id, purchase_token: token }))
    })
    .await
}

pub async fn restore() -> StoreResult<Vec<StorePurchase>> {
    spawn_blocking(|| {
        let json = call_bridge("restore", String::new())?;
        let parsed = serde_json::from_str::<Vec<GoogleTxn>>(&json).map_err(|error| StoreError::Failed(error.to_string()))?;
        Ok(parsed.into_iter().map(store_purchase_from_txn).collect())
    })
    .await
}

pub async fn finish(purchase_token: String) -> StoreResult<()> {
    spawn_blocking(move || {
        call_bridge("acknowledge", purchase_token)?;
        Ok(())
    })
    .await
}

async fn spawn_blocking<T: Send + 'static>(task: impl FnOnce() -> StoreResult<T> + Send + 'static) -> StoreResult<T> {
    tokio::task::spawn_blocking(task).await.map_err(|error| StoreError::Failed(error.to_string()))?
}

fn serde_products(json: &str) -> StoreResult<Vec<StoreProduct>> {
    let mut products = Vec::new();
    for part in json.split('\n').filter(|part| !part.is_empty()) {
        let mut fields = part.split('\t');
        let Some(product_id) = fields.next() else {
            continue;
        };
        products.push(StoreProduct {
            product_id: product_id.to_string(),
            title: fields.next().unwrap_or_default().to_string(),
            description: fields.next().unwrap_or_default().to_string(),
            price_text: fields.next().unwrap_or_default().to_string(),
        });
    }
    Ok(products)
}

fn call_bridge(method: &str, arg: String) -> StoreResult<String> {
    let ctx = ndk_context::android_context();
    let vm = unsafe { JavaVM::from_raw(ctx.vm().cast()) }.map_err(|error| StoreError::NotReady(error.to_string()))?;
    let mut env = vm.attach_current_thread().map_err(|error| StoreError::NotReady(error.to_string()))?;
    let activity = unsafe { JObject::from_raw(ctx.context().cast()) };
    let name = env.new_string(BRIDGE_CLASS).map_err(|error| jni_error(&env, error))?;
    let class = env
        .call_method(&activity, "getAppClass", "(Ljava/lang/String;)Ljava/lang/Class;", &[JValue::Object(&name)])
        .and_then(|value| value.l())
        .map_err(|error| StoreError::NotReady(format!("getAppClass {} failed: {}", BRIDGE_CLASS, jni_message(&env, error))))?;
    let class = JClass::from(class);
    let arg_value = env.new_string(&arg).map_err(|error| jni_error(&env, error))?;
    let result = env
        .call_static_method(class, method, "(Landroid/app/Activity;Ljava/lang/String;)Ljava/lang/String;", &[JValue::Object(&activity), JValue::Object(&arg_value)])
        .map_err(|error| jni_error(&env, error))?;
    let value = result.l().map_err(|error| jni_error(&env, error))?;
    let text: String = env.get_string(&value.into()).map_err(|error| jni_error(&env, error))?.into();
    if let Some(message) = text.strip_prefix("ERROR:") {
        if message == "CANCELLED" {
            return Err(StoreError::Cancelled);
        }
        return Err(StoreError::Failed(message.to_string()));
    }
    Ok(text)
}

fn jni_error(env: &JNIEnv, error: JniError) -> StoreError {
    StoreError::Failed(jni_message(env, error))
}

fn jni_message(env: &JNIEnv, error: JniError) -> String {
    match env.exception_check() {
        Ok(true) => match env.exception_clear() {
            Ok(()) => error.to_string(),
            Err(clear_error) => format!("{}; exception_clear failed: {}", error, clear_error),
        },
        Ok(false) => error.to_string(),
        Err(check_error) => format!("{}; exception_check failed: {}", error, check_error),
    }
}
