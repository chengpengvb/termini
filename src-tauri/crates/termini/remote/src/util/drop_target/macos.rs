use anyhow::Result;
use tauri::Manager;
use block2::DynBlock;
use objc2::rc::Retained;
use std::sync::{Arc, Mutex};
use objc2::runtime::ProtocolObject;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::model::req::req_ws_event::ReqWsEventSftp;
use crate::model::resp::resp_sftp::RespSftpRemoteDrag;
use crate::model::req::req_ws_context::ReqSocketContext;
use crate::web::ws::handle::r#impl::handle_sftp::HandleSftp;
use objc2_app_kit::{NSDraggingSource, NSFilePromiseProviderDelegate};
use crate::model::req::event::ws_sftp::{SftpTransferData, TransferEvent};
use raw_window_handle::{AppKitWindowHandle, HasWindowHandle, RawWindowHandle};
use objc2::{define_class, msg_send, AnyThread, DefinedClass, MainThreadMarker, MainThreadOnly};
use objc2_foundation::{NSArray, NSDate, NSError, NSObject, NSObjectProtocol, NSOperationQueue, NSPoint, NSRect, NSSize, NSString, NSURL};

const FILE_URL_UTI: &str = "public.file-url";
const PLACEHOLDER_FILE_NAME: &str = "DragandDrop.Files";

struct SharedState {
    target_path: Mutex<Option<String>>,
    drag_finished: AtomicBool,
    result_operation: Mutex<objc2_app_kit::NSDragOperation>,
}

struct DelegateIvars {
    work_queue: Retained<NSOperationQueue>,
    shared: Arc<SharedState>,
}

define_class! {
    #[unsafe(super(NSObject))]
    #[thread_kind = AnyThread]
    #[name = "TerminiFilePromiseDelegate"]
    #[ivars = DelegateIvars]
    struct TerminiFilePromiseDelegate;

    unsafe impl NSObjectProtocol for TerminiFilePromiseDelegate {}

    unsafe impl NSFilePromiseProviderDelegate for TerminiFilePromiseDelegate {
        #[unsafe(method_id(filePromiseProvider:fileNameForType:))]
        fn file_name(&self, _p: &objc2_app_kit::NSFilePromiseProvider, _t: &NSString) -> Retained<NSString> {
            NSString::from_str(PLACEHOLDER_FILE_NAME)
        }

        #[unsafe(method(filePromiseProvider:writePromiseToURL:completionHandler:))]
        unsafe fn write_promise(
            &self,
            _p: &objc2_app_kit::NSFilePromiseProvider,
            url: &NSURL,
            completion: &DynBlock<dyn Fn(*mut NSError)>,
        ) {
            if let Some(parent) = url.URLByDeletingLastPathComponent() {
                if let Some(path) = parent.path() {
                    let mut guard = self.ivars().shared.target_path.lock().unwrap();
                    if guard.is_none() {
                        *guard = Some(path.to_string());
                    }
                }
            }
            completion.call((std::ptr::null_mut(),));
        }

        #[unsafe(method_id(operationQueueForFilePromiseProvider:))]
        fn op_queue(&self, _p: &objc2_app_kit::NSFilePromiseProvider) -> Retained<NSOperationQueue> {
            Retained::clone(&self.ivars().work_queue)
        }
    }
}

struct SourceIvars {
    shared: Arc<SharedState>,
}

define_class! {
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "TerminiRemoteDragSource"]
    #[ivars = SourceIvars]
    struct TerminiRemoteDragSource;

    unsafe impl NSObjectProtocol for TerminiRemoteDragSource {}

    unsafe impl NSDraggingSource for TerminiRemoteDragSource {
        #[unsafe(method(draggingSession:sourceOperationMaskForDraggingContext:))]
        fn op_mask(
            &self,
            _session: &objc2_app_kit::NSDraggingSession,
            context: objc2_app_kit::NSDraggingContext,
        ) -> objc2_app_kit::NSDragOperation {
            if context == objc2_app_kit::NSDraggingContext::OutsideApplication {
                objc2_app_kit::NSDragOperation::Copy
            } else {
                objc2_app_kit::NSDragOperation::None
            }
        }

        #[unsafe(method(draggingSession:endedAtPoint:operation:))]
        fn session_ended(
            &self,
            _session: &objc2_app_kit::NSDraggingSession,
            _pt: NSPoint,
            operation: objc2_app_kit::NSDragOperation,
        ) {
            *self.ivars().shared.result_operation.lock().unwrap() = operation;
            self.ivars().shared.drag_finished.store(true, Ordering::Release);
        }
    }
}

pub fn start_remote_drag(handle: tauri::AppHandle, sftp_handler: HandleSftp, req: RespSftpRemoteDrag) -> Result<()> {
    let rt = tokio::runtime::Handle::current();
    let err = Arc::new(Mutex::new(None::<String>));
    let err_for_main = Arc::clone(&err);
    let handle_for_drag = handle.clone();
    handle
        .run_on_main_thread(move || {
            if let Err(e) = unsafe { do_start_drag(handle_for_drag, sftp_handler, req, rt) } {
                *err_for_main.lock().unwrap() = Some(e.to_string());
            }
        })
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    if let Some(msg) = err.lock().map_err(|e| anyhow::anyhow!("{e}"))?.take() {
        return Err(anyhow::anyhow!(msg));
    }
    Ok(())
}

unsafe fn do_start_drag(handle: tauri::AppHandle, sftp_handler: HandleSftp, req: RespSftpRemoteDrag, rt: tokio::runtime::Handle) -> Result<()> {
    let mtm = MainThreadMarker::new().ok_or_else(|| anyhow::anyhow!("remote drag must run on main thread"))?;

    let window = handle.get_webview_window("main").ok_or_else(|| anyhow::anyhow!("main window not found"))?;
    let wh = window.window_handle().map_err(|e| anyhow::anyhow!("{e}"))?;
    let app_kit_handle: AppKitWindowHandle = match wh.as_raw() {
        RawWindowHandle::AppKit(h) => h,
        _ => return Err(anyhow::anyhow!("not an AppKit window")),
    };
    let view: &objc2_app_kit::NSView = &*(app_kit_handle.ns_view.as_ptr() as *const objc2_app_kit::NSView);
    let ns_window: Retained<objc2_app_kit::NSWindow> = view.window().ok_or_else(|| anyhow::anyhow!("drag source view has no window"))?;

    let event = resolve_drag_event(&ns_window, req.screen_x, req.screen_y)?;
    let window_point = event.locationInWindow();
    let view_point = view.convertPoint_fromView(window_point, None);

    let shared = Arc::new(SharedState { target_path: Mutex::new(None), drag_finished: AtomicBool::new(false), result_operation: Mutex::new(objc2_app_kit::NSDragOperation::None) });

    let work_queue = NSOperationQueue::new();
    work_queue.setQualityOfService(objc2_foundation::NSQualityOfService::UserInitiated);

    let promise_delegate: Retained<TerminiFilePromiseDelegate> = {
        let obj = TerminiFilePromiseDelegate::alloc().set_ivars(DelegateIvars { work_queue, shared: Arc::clone(&shared) });
        msg_send![super(obj), init]
    };

    let file_type = NSString::from_str(FILE_URL_UTI);
    let provider = objc2_app_kit::NSFilePromiseProvider::initWithFileType_delegate(objc2_app_kit::NSFilePromiseProvider::alloc(), &file_type, ProtocolObject::from_ref(&*promise_delegate));
    let drag_item = objc2_app_kit::NSDraggingItem::initWithPasteboardWriter(objc2_app_kit::NSDraggingItem::alloc(), ProtocolObject::from_ref(&*provider));
    drag_item.setDraggingFrame_contents(NSRect { origin: view_point, size: NSSize { width: 16.0, height: 16.0 } }, None);

    let source: Retained<TerminiRemoteDragSource> = {
        let obj = TerminiRemoteDragSource::alloc(mtm).set_ivars(SourceIvars { shared: Arc::clone(&shared) });
        msg_send![super(obj), init]
    };

    let items = NSArray::from_retained_slice(&[drag_item]);
    view.beginDraggingSessionWithItems_event_source(&items, &event, ProtocolObject::from_ref(&*source));

    pump_until_drag_finished(&shared);

    let operation = *shared.result_operation.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
    if operation == objc2_app_kit::NSDragOperation::None {
        return Ok(());
    }

    let target_path = match shared.target_path.lock().map_err(|e| anyhow::anyhow!("{e}"))?.clone() {
        Some(path) => path,
        None => return Ok(()),
    };
    if is_temp_drop_path(&target_path) {
        return Ok(());
    }

    let data = SftpTransferData {
        target_path,
        names: req.names,
        source_path: req.source_path,
        side_session_id: req.session_id.clone(),
        source_client_transfer_id: req.client_transfer_id.clone(),
        target_client_transfer_id: req.local_client_transfer_id.clone(),
    };
    let session_id = req.session_id;
    rt.spawn(async move {
        let payload = serde_json::to_value(TransferEvent { data: data.clone() }).ok();
        let request = Arc::new(ReqSocketContext { r#type: ReqWsEventSftp::Transfer as i16, session_id: session_id.clone(), payload });
        let result = match sftp_handler.resolve_socket_by_session(&session_id) {
            Ok(socket) => sftp_handler.do_execute_transfer(socket, request, data, None).await,
            Err(error) => Err(error),
        };
        if let Err(e) = result {
            summer::tracing::error!("[file] drag transfer failed: {e}");
        }
    });

    Ok(())
}

unsafe fn resolve_drag_event(window: &objc2_app_kit::NSWindow, screen_x: f64, screen_y: f64) -> Result<Retained<objc2_app_kit::NSEvent>> {
    let mtm = MainThreadMarker::new().ok_or_else(|| anyhow::anyhow!("remote drag must run on main thread"))?;
    let app = objc2_app_kit::NSApplication::sharedApplication(mtm);
    if let Some(event) = app.currentEvent() {
        return Ok(event);
    }

    let screen_point = NSPoint { x: screen_x, y: screen_y };
    let window_point = window.convertPointFromScreen(screen_point);
    #[rustfmt::skip]
    let event = objc2_app_kit::NSEvent::mouseEventWithType_location_modifierFlags_timestamp_windowNumber_context_eventNumber_clickCount_pressure(objc2_app_kit::NSEventType::LeftMouseDragged, window_point, objc2_app_kit::NSEventModifierFlags(0), 0.0, window.windowNumber(), None, 0, 1, 0.0).ok_or_else(|| anyhow::anyhow!("failed to synthesize drag event"))?;
    Ok(event)
}

fn pump_until_drag_finished(shared: &SharedState) {
    while !shared.drag_finished.load(Ordering::Acquire) {
        unsafe {
            let pool = objc2_foundation::NSAutoreleasePool::new();
            let until = NSDate::dateWithTimeIntervalSinceNow(0.01);
            let run_loop = objc2_foundation::NSRunLoop::currentRunLoop();
            let mode = objc2_foundation::NSDefaultRunLoopMode;
            run_loop.runMode_beforeDate(mode, &until);
            drop(pool);
        }
    }
}

fn is_temp_drop_path(path: &str) -> bool {
    let temp = std::env::temp_dir();
    let temp_str = temp.to_string_lossy();
    path.starts_with(temp_str.as_ref())
}
