use std::sync::mpsc;
use std::time::Duration;
use std::ffi::{c_char, c_void, CStr, CString};
use crate::txn::{store_purchase_from_txn, AppleTxn};
use store_core::{StoreError, StoreProduct, StorePurchase, StoreResult};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const PURCHASE_TIMEOUT: Duration = Duration::from_secs(300);

type BridgeCallback = unsafe extern "C" fn(context: *mut c_void, code: i32, payload: *const c_char);

extern "C" {
    fn store_bridge_list_products(ids_json: *const c_char, context: *mut c_void, callback: BridgeCallback);
    fn store_bridge_purchase(product_id: *const c_char, context: *mut c_void, callback: BridgeCallback);
    fn store_bridge_finish(original_id: *const c_char, context: *mut c_void, callback: BridgeCallback);
    fn store_bridge_unfinished(context: *mut c_void, callback: BridgeCallback);
    fn store_bridge_restore(context: *mut c_void, callback: BridgeCallback);
    fn store_bridge_listen(context: *mut c_void, callback: BridgeCallback);
}

struct BridgeReply {
    code: i32,
    payload: String,
}

unsafe fn read_payload(payload: *const c_char) -> String {
    match payload.is_null() {
        true => String::new(),
        false => CStr::from_ptr(payload).to_string_lossy().into_owned(),
    }
}

unsafe extern "C" fn bridge_callback(context: *mut c_void, code: i32, payload: *const c_char) {
    let sender = Box::from_raw(context as *mut mpsc::Sender<BridgeReply>);
    let _ = sender.send(BridgeReply { code, payload: read_payload(payload) });
}

unsafe extern "C" fn listen_callback(context: *mut c_void, code: i32, payload: *const c_char) {
    let sender = &*(context as *const UnboundedSender<StoreResult<StorePurchase>>);
    let payload = read_payload(payload);
    let reply = match code {
        0 => parse_purchase(&payload),
        _ => Err(StoreError::Failed(payload)),
    };
    let _ = sender.send(reply);
}

fn bridge_call(timeout: Duration, invoke: impl FnOnce(*mut c_void, BridgeCallback)) -> StoreResult<String> {
    let (sender, receiver) = mpsc::channel::<BridgeReply>();
    let context = Box::into_raw(Box::new(sender)) as *mut c_void;
    invoke(context, bridge_callback);
    let reply = receiver.recv_timeout(timeout).map_err(|_| StoreError::Failed("store request timed out".to_string()))?;
    match reply.code {
        0 => Ok(reply.payload),
        1 => Err(StoreError::Cancelled),
        _ => Err(StoreError::Failed(reply.payload)),
    }
}

fn parse_purchase(payload: &str) -> StoreResult<StorePurchase> {
    let parsed = serde_json::from_str::<AppleTxn>(payload).map_err(|error| StoreError::Failed(error.to_string()))?;
    Ok(store_purchase_from_txn(parsed))
}

fn parse_purchases(payload: &str) -> StoreResult<Vec<StorePurchase>> {
    let parsed = serde_json::from_str::<Vec<AppleTxn>>(payload).map_err(|error| StoreError::Failed(error.to_string()))?;
    Ok(parsed.into_iter().map(store_purchase_from_txn).collect())
}

pub async fn list_products(product_ids: Vec<String>) -> StoreResult<Vec<StoreProduct>> {
    spawn_blocking(move || {
        let ids_json = serde_json::to_string(&product_ids).map_err(|error| StoreError::Failed(error.to_string()))?;
        let ids = CString::new(ids_json).map_err(|error| StoreError::Failed(error.to_string()))?;
        let payload = bridge_call(REQUEST_TIMEOUT, |context, callback| unsafe { store_bridge_list_products(ids.as_ptr(), context, callback) })?;
        serde_json::from_str(&payload).map_err(|error| StoreError::Failed(error.to_string()))
    })
    .await
}

pub async fn purchase(_hwnd: isize, product_id: String) -> StoreResult<StorePurchase> {
    spawn_blocking(move || {
        let id = CString::new(product_id).map_err(|error| StoreError::Failed(error.to_string()))?;
        let payload = bridge_call(PURCHASE_TIMEOUT, |context, callback| unsafe { store_bridge_purchase(id.as_ptr(), context, callback) })?;
        parse_purchase(&payload)
    })
    .await
}

pub async fn finish(original_transaction_id: String) -> StoreResult<()> {
    spawn_blocking(move || {
        let id = CString::new(original_transaction_id).map_err(|error| StoreError::Failed(error.to_string()))?;
        bridge_call(REQUEST_TIMEOUT, |context, callback| unsafe { store_bridge_finish(id.as_ptr(), context, callback) })?;
        Ok(())
    })
    .await
}

pub async fn unfinished() -> StoreResult<Vec<StorePurchase>> {
    spawn_blocking(|| parse_purchases(&bridge_call(REQUEST_TIMEOUT, |context, callback| unsafe { store_bridge_unfinished(context, callback) })?)).await
}

pub async fn restore() -> StoreResult<Vec<StorePurchase>> {
    spawn_blocking(|| parse_purchases(&bridge_call(PURCHASE_TIMEOUT, |context, callback| unsafe { store_bridge_restore(context, callback) })?)).await
}

pub fn updates() -> UnboundedReceiver<StoreResult<StorePurchase>> {
    let (sender, receiver) = unbounded_channel::<StoreResult<StorePurchase>>();
    let context = Box::into_raw(Box::new(sender)) as *mut c_void;
    unsafe { store_bridge_listen(context, listen_callback) };
    receiver
}

async fn spawn_blocking<T: Send + 'static>(task: impl FnOnce() -> StoreResult<T> + Send + 'static) -> StoreResult<T> {
    tokio::task::spawn_blocking(task).await.map_err(|error| StoreError::Failed(error.to_string()))?
}
