use block2::RcBlock;
use std::sync::Mutex;
use objc2::MainThreadMarker;
use summer::tracing::{info, warn};
use objc2_ui_kit::{UIApplication, UIBackgroundTaskIdentifier, UIBackgroundTaskInvalid};

static TASK: Mutex<Option<UIBackgroundTaskIdentifier>> = Mutex::new(None);

pub fn begin() {
    let Some(mtm) = MainThreadMarker::new() else {
        warn!("[ios-bg] begin 不在主线程, 跳过");
        return;
    };
    let mut slot = TASK.lock().expect("ios background task lock poisoned");
    if slot.is_some() {
        return;
    }
    let app = UIApplication::sharedApplication(mtm);
    let handler = RcBlock::new(|| {
        info!("[ios-bg] 后台保活到期");
        end();
    });
    let id = app.beginBackgroundTaskWithExpirationHandler(Some(&*handler));
    if id == unsafe { UIBackgroundTaskInvalid } {
        warn!("[ios-bg] 系统拒绝后台任务");
        return;
    }
    *slot = Some(id);
    info!("[ios-bg] 后台保活开始 id={id}");
}

pub fn end() {
    let Some(mtm) = MainThreadMarker::new() else {
        warn!("[ios-bg] end 不在主线程, 跳过");
        return;
    };
    let taken = TASK.lock().expect("ios background task lock poisoned").take();
    let Some(id) = taken else {
        return;
    };
    UIApplication::sharedApplication(mtm).endBackgroundTask(id);
    info!("[ios-bg] 后台保活结束 id={id}");
}
