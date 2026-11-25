use std::sync::Arc;
use std::sync::OnceLock;
use tokio::process::Command;
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use crate::module::common::context::context_shell::ShellDescr;

struct ShellCache {
    list: Vec<ShellDescr>,
    id_2_descr: HashMap<String, ShellDescr>,
}

impl ShellCache {
    fn new(list: Vec<ShellDescr>) -> Self {
        let mut by_id: HashMap<String, ShellDescr> = HashMap::new();
        for descr in list.iter() {
            if by_id.contains_key(descr.shell_id.as_str()) {
                continue;
            }
            by_id.insert(descr.shell_id.clone(), descr.clone());
        }
        Self { list, id_2_descr: by_id }
    }
}

static SHELL_CACHE: OnceLock<ShellCache> = OnceLock::new();

pub trait ShellDiscover: Send + Sync {
    fn discover(&self) -> Vec<ShellDescr>;

    fn build_exec_args(&self, descr: &ShellDescr, command: &str) -> Result<Vec<String>>;

    fn discover_cached(&self) -> Vec<ShellDescr> {
        let cache = SHELL_CACHE.get_or_init(|| ShellCache::new(self.discover()));
        cache.list.clone()
    }

    fn find_by_shell_id(&self, shell_id: &str) -> Option<ShellDescr> {
        let cache = SHELL_CACHE.get_or_init(|| ShellCache::new(self.discover()));
        cache.id_2_descr.get(shell_id).cloned()
    }
}

#[cfg(target_os = "windows")]
pub fn native_shell_platform() -> Arc<dyn ShellDiscover + Send + Sync> {
    Arc::new(crate::module::shell::platform::windows::shell_discover_windows::ShellDiscoverWindows::new())
}

#[cfg(target_os = "linux")]
pub fn native_shell_platform() -> Arc<dyn ShellDiscover + Send + Sync> {
    Arc::new(crate::module::shell::platform::linux::shell_discover_linux::ShellDiscoverLinux::new())
}

#[cfg(target_os = "macos")]
pub fn native_shell_platform() -> Arc<dyn ShellDiscover + Send + Sync> {
    Arc::new(crate::module::shell::platform::macos::shell_discover_macos::ShellDiscoverMacos::new())
}

#[cfg(target_os = "android")]
pub fn native_shell_platform() -> Arc<dyn ShellDiscover + Send + Sync> {
    Arc::new(crate::module::shell::platform::android::shell_discover_android::ShellDiscoverAndroid::new())
}

#[cfg(target_os = "ios")]
pub fn native_shell_platform() -> Arc<dyn ShellDiscover + Send + Sync> {
    Arc::new(crate::module::shell::platform::ios::shell_discover_ios::ShellDiscoverIos::new())
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos", target_os = "android", target_os = "ios")))]
compile_error!("[shell] shell module only supports windows / linux / macos / android / ios targets");

#[cfg(any(windows, target_os = "android"))]
pub fn unknown_exec_shell(shell_id: &str) -> anyhow::Error {
    anyhow!("headless execution of this shell is not supported on the current platform: {}", shell_id)
}

#[cfg(unix)]
pub fn spawn_in_own_group(cmd: &mut Command) {
    cmd.process_group(0);
}

#[cfg(windows)]
pub fn spawn_in_own_group(_cmd: &mut Command) {}

#[cfg(unix)]
pub fn limit_cpu_rate(child: &tokio::process::Child, percent: i16) -> Result<()> {
    if percent <= 0 || percent >= 100 {
        return Ok(());
    }
    let Some(pid) = child.id() else {
        return Err(anyhow!("the command has no pid, cpu limit was not applied"));
    };
    let nice = (i32::from(100 - percent) * 19 / 90).clamp(1, 19);
    if unsafe { libc::setpriority(libc::PRIO_PGRP, pid, nice) } != 0 {
        return Err(anyhow!("failed to lower priority pid={} nice={} error={}", pid, nice, std::io::Error::last_os_error()));
    }
    Ok(())
}

#[cfg(windows)]
pub fn limit_cpu_rate(child: &tokio::process::Child, percent: i16) -> Result<()> {
    use windows_sys::Win32::Foundation::HANDLE;
    use std::sync::atomic::{AtomicI16, Ordering};
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JobObjectCpuRateControlInformation, SetInformationJobObject, JOBOBJECT_CPU_RATE_CONTROL_INFORMATION, JOB_OBJECT_CPU_RATE_CONTROL_ENABLE,
        JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP,
    };

    static SHELL_JOB: OnceLock<usize> = OnceLock::new();
    static APPLIED_PERCENT: AtomicI16 = AtomicI16::new(0);

    if percent <= 0 || percent >= 100 {
        return Ok(());
    }
    let Some(handle) = child.raw_handle() else {
        return Err(anyhow!("the command has no process handle, cpu limit was not applied"));
    };
    let job = *SHELL_JOB.get_or_init(|| unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) } as usize);
    if job == 0 {
        return Err(anyhow!("failed to create the shell job object: {}", std::io::Error::last_os_error()));
    }
    let job = job as HANDLE;
    if APPLIED_PERCENT.swap(percent, Ordering::Relaxed) != percent {
        let mut info: JOBOBJECT_CPU_RATE_CONTROL_INFORMATION = unsafe { std::mem::zeroed() };
        info.ControlFlags = JOB_OBJECT_CPU_RATE_CONTROL_ENABLE | JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP;
        info.Anonymous.CpuRate = u32::from(percent as u16) * 100;
        let size = std::mem::size_of::<JOBOBJECT_CPU_RATE_CONTROL_INFORMATION>() as u32;
        let ok = unsafe { SetInformationJobObject(job, JobObjectCpuRateControlInformation, std::ptr::addr_of!(info).cast(), size) };
        if ok == 0 {
            return Err(anyhow!("failed to set the cpu hard cap to {}%: {}", percent, std::io::Error::last_os_error()));
        }
    }
    if unsafe { AssignProcessToJobObject(job, handle as HANDLE) } == 0 {
        return Err(anyhow!("failed to put the command into the shell job object: {}", std::io::Error::last_os_error()));
    }
    Ok(())
}

#[cfg(unix)]
pub async fn kill_process_tree(pid: u32) -> Result<()> {
    if unsafe { libc::killpg(pid as libc::pid_t, libc::SIGKILL) } != 0 {
        return Err(anyhow!("failed to kill process group pid={} error={}", pid, std::io::Error::last_os_error()));
    }
    Ok(())
}

#[cfg(windows)]
pub async fn kill_process_tree(pid: u32) -> Result<()> {
    use crate::util::ssh_shell::shell_text::decode_shell_bytes;

    // 主程序是 GUI 子系统, taskkill 若自带控制台会闪出黑窗
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let output = Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .await
        .map_err(|error| anyhow!("failed to invoke taskkill pid={}: {}", pid, error))?;
    if !output.status.success() {
        return Err(anyhow!("taskkill did not terminate the process tree pid={} output={}", pid, decode_shell_bytes(&output.stderr).trim()));
    }
    Ok(())
}
