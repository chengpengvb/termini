use std::path::PathBuf;
use std::sync::OnceLock;
use summer::tracing::warn;
use anyhow::{anyhow, Result};
use crate::module::common::context::context_shell::ShellDescr;
use crate::module::shell::platform::shell_discover::ShellDiscover;

static NATIVE_LIBRARY_DIR: OnceLock<Option<PathBuf>> = OnceLock::new();

fn native_library_dir() -> Option<&'static PathBuf> {
    NATIVE_LIBRARY_DIR
        .get_or_init(|| match resolve_native_library_dir() {
            Ok(dir) => Some(dir),
            Err(error) => {
                warn!("[ai] 解析 nativeLibraryDir 失败, 内置 shell 不可用: {}", error);
                None
            }
        })
        .as_ref()
}

fn resolve_native_library_dir() -> Result<PathBuf> {
    use jni::JavaVM;
    use jni::objects::JObject;

    let ctx = ndk_context::android_context();
    let vm = unsafe { JavaVM::from_raw(ctx.vm().cast()) }.map_err(|error| anyhow!("{}", error))?;
    let mut env = vm.attach_current_thread().map_err(|error| anyhow!("{}", error))?;
    let context = unsafe { JObject::from_raw(ctx.context().cast()) };
    let info = env
        .call_method(&context, "getApplicationInfo", "()Landroid/content/pm/ApplicationInfo;", &[])
        .map_err(|error| anyhow!("getApplicationInfo: {}", error))?
        .l()
        .map_err(|error| anyhow!("{}", error))?;
    let value = env
        .get_field(&info, "nativeLibraryDir", "Ljava/lang/String;")
        .map_err(|error| anyhow!("nativeLibraryDir: {}", error))?
        .l()
        .map_err(|error| anyhow!("{}", error))?;
    let text: String = env.get_string(&value.into()).map_err(|error| anyhow!("{}", error))?.into();
    if text.is_empty() {
        return Err(anyhow!("nativeLibraryDir is empty"));
    }
    Ok(PathBuf::from(text))
}

pub struct ShellDiscoverAndroid;

impl ShellDiscoverAndroid {
    pub fn new() -> Self {
        Self
    }

    fn system_sh(&self) -> Option<ShellDescr> {
        let path = PathBuf::from("/system/bin/sh");
        if !path.is_file() {
            return None;
        }
        Some(ShellDescr { shell_id: "android.sh".to_owned(), description: "Android system sh (/system/bin/sh)".to_owned(), exe: Some(path), args: Vec::new(), env: Vec::new(), supports_osc633: true })
    }

    fn jni_lib_shell(&self, lib_name: &str, shell_id: &str, description: &str, args: Vec<String>) -> Option<ShellDescr> {
        let path = native_library_dir()?.join(lib_name);
        if !path.is_file() {
            return None;
        }
        Some(ShellDescr { shell_id: shell_id.to_owned(), description: description.to_owned(), exe: Some(path), args, env: Vec::new(), supports_osc633: true })
    }

    fn jni_bash(&self) -> Option<ShellDescr> {
        self.jni_lib_shell("libtermini_bash.so", "android.bash", "Android Bash (bundled)", vec!["--login".to_owned()])
    }

    fn jni_busybox(&self) -> Option<ShellDescr> {
        self.jni_lib_shell("libtermini_busybox.so", "android.busybox", "Android BusyBox sh (bundled)", vec!["sh".to_owned()])
    }
}

impl ShellDiscover for ShellDiscoverAndroid {
    fn discover(&self) -> Vec<ShellDescr> {
        let mut result: Vec<ShellDescr> = Vec::new();
        if let Some(descriptor) = self.jni_bash() {
            result.push(descriptor);
        }
        if let Some(descriptor) = self.jni_busybox() {
            result.push(descriptor);
        }
        if let Some(descriptor) = self.system_sh() {
            result.push(descriptor);
        }
        result
    }

    fn build_exec_args(&self, descr: &ShellDescr, command: &str) -> anyhow::Result<Vec<String>> {
        match descr.shell_id.as_str() {
            "android.sh" | "android.bash" => Ok(vec!["-c".to_owned(), command.to_owned()]),
            "android.busybox" => Ok(vec!["sh".to_owned(), "-c".to_owned(), command.to_owned()]),
            other => Err(crate::module::shell::platform::shell_discover::unknown_exec_shell(other)),
        }
    }
}
