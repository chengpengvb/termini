use anyhow::{anyhow, Result};

pub fn open_local_file(path: &str, open_with: bool) -> Result<()> {
    #[cfg(target_os = "android")]
    {
        return open_android(path, open_with);
    }
    #[cfg(target_os = "ios")]
    {
        let _ = open_with;
        return super::file_open_ios::present_open_in(path);
    }
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        if !open_with {
            return tauri_plugin_opener::open_path(path, None::<&str>).map_err(|e| anyhow!("open_path: {}", e));
        }
        open_with_picker(path)
    }
}

pub fn reveal_local_file(path: &str) -> Result<()> {
    tauri_plugin_opener::reveal_item_in_dir(path).map_err(|e| anyhow!("reveal_item_in_dir: {}", e))
}

#[cfg(target_os = "windows")]
fn open_with_picker(path: &str) -> Result<()> {
    std::process::Command::new("rundll32.exe").args(["shell32.dll,OpenAs_RunDLL", path]).spawn().map_err(|e| anyhow!("open_with_dialog: {}", e))?;
    Ok(())
}

#[cfg(all(not(target_os = "windows"), not(target_os = "android"), not(target_os = "ios")))]
fn open_with_picker(path: &str) -> Result<()> {
    tauri_plugin_opener::open_path(path, None::<&str>).map_err(|e| anyhow!("open_path fallback: {}", e))
}

#[cfg(target_os = "android")]
fn open_android(path: &str, open_with: bool) -> Result<()> {
    use jni::JavaVM;
    use jni::objects::{JObject, JValue};
    let ctx = ndk_context::android_context();
    let vm = unsafe { JavaVM::from_raw(ctx.vm().cast()) }.map_err(|e| anyhow!("{}", e))?;
    let mut env = vm.attach_current_thread().map_err(|e| anyhow!("{}", e))?;
    let activity = unsafe { JObject::from_raw(ctx.context().cast()) };
    let class = env.find_class("com/termini/www/MainActivity").map_err(|e| anyhow!("{}", e))?;
    let path_j = env.new_string(path).map_err(|e| anyhow!("{}", e))?;
    let result = env
        .call_static_method(
            class,
            "openLocalFile",
            "(Landroid/app/Activity;Ljava/lang/String;Z)Ljava/lang/String;",
            &[JValue::Object(&activity), JValue::Object(&path_j), JValue::Bool(if open_with { 1u8 } else { 0u8 })],
        )
        .map_err(|e| anyhow!("{}", e))?;
    let value = result.l().map_err(|e| anyhow!("{}", e))?;
    let text: String = env.get_string(&value.into()).map_err(|e| anyhow!("{}", e))?.into();
    if text == "OK" {
        return Ok(());
    }
    if let Some(message) = text.strip_prefix("ERROR:") {
        return Err(anyhow!("{}", message));
    }
    Err(anyhow!("{}", text))
}
