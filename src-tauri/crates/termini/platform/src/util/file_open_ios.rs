use dispatch::Queue;
use std::path::Path;
use objc2::rc::Retained;
use objc2::MainThreadOnly;
use objc2::MainThreadMarker;
use anyhow::{anyhow, Result};
use objc2::runtime::AnyObject;
use objc2_foundation::{NSArray, NSString, NSURL};
use objc2_ui_kit::{UIActivityViewController, UIApplication, UIViewController, UIWindow, UIWindowScene};

pub fn present_open_in(path: &str) -> Result<()> {
    let path = normalize_path(path);
    if !Path::new(&path).exists() {
        return Err(anyhow!("file not found: {}", path));
    }
    match MainThreadMarker::new() {
        Some(mtm) => present_on_main(&path, mtm),
        None => Queue::main().exec_sync(move || {
            let mtm = unsafe { MainThreadMarker::new_unchecked() };
            present_on_main(&path, mtm)
        }),
    }
}

fn normalize_path(path: &str) -> String {
    let trimmed = path.trim();
    if trimmed.starts_with("file://") {
        if let Ok(url) = url::Url::parse(trimmed) {
            if let Ok(file_path) = url.to_file_path() {
                return file_path.to_string_lossy().into_owned();
            }
        }
    }
    trimmed.to_string()
}

fn present_on_main(path: &str, mtm: MainThreadMarker) -> Result<()> {
    let presenter = top_view_controller(mtm)?;
    let ns_path = NSString::from_str(path);
    let file_url = NSURL::fileURLWithPath_isDirectory(&ns_path, Path::new(path).is_dir());
    let item: Retained<AnyObject> = unsafe { Retained::cast_unchecked(file_url) };
    let items = NSArray::from_retained_slice(&[item]);
    let activity = unsafe { UIActivityViewController::initWithActivityItems_applicationActivities(UIActivityViewController::alloc(mtm), &items, None) };
    presenter.presentViewController_animated_completion(&activity, true, None);
    Ok(())
}

fn top_view_controller(mtm: MainThreadMarker) -> Result<Retained<UIViewController>> {
    let app = UIApplication::sharedApplication(mtm);
    let window = key_window(&app).ok_or_else(|| anyhow!("no window to present Open In sheet"))?;
    let mut current = window.rootViewController().ok_or_else(|| anyhow!("no view controller to present Open In sheet"))?;
    while let Some(presented) = current.presentedViewController() {
        current = presented;
    }
    Ok(current)
}

fn key_window(app: &UIApplication) -> Option<Retained<UIWindow>> {
    let scenes = app.connectedScenes().allObjects();
    let mut fallback = None;
    for i in 0..scenes.count() {
        let Ok(window_scene) = scenes.objectAtIndex(i).downcast::<UIWindowScene>() else {
            continue;
        };
        if let Some(window) = window_scene.keyWindow() {
            return Some(window);
        }
        let windows = window_scene.windows();
        for j in 0..windows.count() {
            let window = windows.objectAtIndex(j);
            if window.isKeyWindow() {
                return Some(window);
            }
            if fallback.is_none() {
                fallback = Some(window);
            }
        }
    }
    fallback
}
