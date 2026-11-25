use std::io::Read;
use std::sync::Once;
use parking_lot::Mutex;
use std::sync::OnceLock;
use crate::util::workspace;
use std::os::fd::FromRawFd;
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use objc2_foundation::NSString;
use objc2::rc::autoreleasepool;
use std::ffi::{c_char, c_int, c_void, CString};
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};

#[link(name = "ios_system", kind = "framework")]
extern "C" {
    fn ios_kill() -> c_int;
    fn initializeEnvironment();
    fn ios_setContext(context: *const c_void);
    fn ios_closeSession(session: *const c_void);
    fn ios_switchSession(session: *const c_void);
    fn ios_system(command: *const c_char) -> c_int;
    fn ios_setMiniRoot(path: *const NSString) -> c_int;
    fn ios_getLogicalPWD(session: *const c_void) -> *mut NSString;
    fn ios_setStreams(input: *mut libc::FILE, output: *mut libc::FILE, error: *mut libc::FILE);
}

static INIT: Once = Once::new();
static RUN_LOCK: Mutex<()> = Mutex::new(());
static DEV_NULL: OnceLock<usize> = OnceLock::new();
static SESSION_SEQ: AtomicUsize = AtomicUsize::new(1);
static SESSION_KEYS: Mutex<Option<HashMap<usize, usize>>> = Mutex::new(None);

pub fn new_session() -> usize {
    SESSION_SEQ.fetch_add(1, Ordering::Relaxed)
}

pub fn session_dir(configured: Option<String>) -> Result<String> {
    if let Some(dir) = configured.map(|value| value.trim().to_string()).filter(|value| !value.is_empty()) {
        return Ok(dir);
    }
    let dir = workspace::workspace_dir()?;
    std::fs::create_dir_all(&dir).map_err(|error| anyhow!("create workspace dir failed: {}: {}", dir.display(), error))?;
    dir.to_str().map(str::to_string).ok_or_else(|| anyhow!("non-UTF-8 workspace path"))
}

pub fn enter(session: usize, cwd: &str) -> Result<()> {
    let output = capture(session, &format!("cd {}", quote(cwd)))?;
    if !output.is_empty() {
        return Err(anyhow!("{}", output));
    }
    Ok(())
}

pub fn initialize(mini_root: &str) -> Result<()> {
    let root = NSString::from_str(mini_root);
    INIT.call_once(|| unsafe {
        initializeEnvironment();
        ios_setMiniRoot(&*root);
    });
    Ok(())
}

pub fn interrupt() {
    unsafe { ios_kill() };
}

pub fn close_session(session: usize) {
    let key = session_key(session);
    let _guard = RUN_LOCK.lock();
    unsafe { ios_closeSession(key) };
}

pub fn logical_pwd(session: usize) -> Result<String> {
    let key = session_key(session);
    let _guard = RUN_LOCK.lock();
    autoreleasepool(|_| {
        let path = unsafe { ios_getLogicalPWD(key) };
        if path.is_null() {
            return Err(anyhow!("ios_system session {} has no working directory", session));
        }
        Ok(unsafe { &*path }.to_string())
    })
}

pub fn run(session: usize, command: &str, chunk_tx: &UnboundedSender<Vec<u8>>) -> Result<i32> {
    let line = CString::new(command).map_err(|error| anyhow!("ios_system command contains NUL: {}", error))?;
    let mut fds: [c_int; 2] = [0; 2];
    if unsafe { libc::pipe(fds.as_mut_ptr()) } != 0 {
        return Err(anyhow!("failed to create ios_system output pipe: {}", std::io::Error::last_os_error()));
    }
    let (read_fd, write_fd) = (fds[0], fds[1]);
    let reader_tx = chunk_tx.clone();
    let reader = std::thread::spawn(move || pump(read_fd, reader_tx));

    let mode = CString::new("w").map_err(|error| anyhow!("ios_system stream mode contains NUL: {}", error))?;
    let stream = unsafe { libc::fdopen(write_fd, mode.as_ptr()) };
    if stream.is_null() {
        unsafe { libc::close(write_fd) };
        reader.join().map_err(|_| anyhow!("ios_system output reader panicked"))?;
        return Err(anyhow!("failed to open ios_system output stream: {}", std::io::Error::last_os_error()));
    }
    unsafe { libc::setvbuf(stream, std::ptr::null_mut(), libc::_IONBF, 0) };

    let result = dispatch(session, line, stream as usize);
    unsafe { libc::fclose(stream) };
    reader.join().map_err(|_| anyhow!("ios_system output reader panicked"))?;
    result
}

pub fn capture(session: usize, command: &str) -> Result<String> {
    let (chunk_tx, mut chunk_rx) = unbounded_channel::<Vec<u8>>();
    run(session, command, &chunk_tx)?;
    drop(chunk_tx);
    let mut output = Vec::new();
    while let Ok(chunk) = chunk_rx.try_recv() {
        output.extend_from_slice(&chunk);
    }
    Ok(String::from_utf8_lossy(&output).trim().to_string())
}

pub fn quote(path: &str) -> String {
    format!("'{}'", path.replace('\'', "'\\''"))
}

fn dispatch(session: usize, line: CString, stream: usize) -> Result<i32> {
    let key = session_key(session) as usize;
    let _guard = RUN_LOCK.lock();
    let worker = std::thread::Builder::new()
        .name("ios-system".to_string())
        .spawn(move || unsafe {
            let stream = stream as *mut libc::FILE;
            ios_switchSession(key as *const c_void);
            ios_setContext(key as *const c_void);
            ios_setStreams(dev_null(), stream, stream);
            ios_system(line.as_ptr())
        })
        .map_err(|error| anyhow!("failed to spawn ios_system thread: {}", error))?;
    worker.join().map_err(|_| anyhow!("ios_system worker panicked"))
}

fn session_key(session: usize) -> *const c_void {
    let mut guard = SESSION_KEYS.lock();
    let keys = guard.get_or_insert_with(HashMap::new);
    if let Some(key) = keys.get(&session) {
        return *key as *const c_void;
    }
    let key = CString::new(format!("termini.{}", session)).expect("session name contains NUL").into_raw() as usize;
    keys.insert(session, key);
    key as *const c_void
}

fn dev_null() -> *mut libc::FILE {
    let stream = DEV_NULL.get_or_init(|| {
        let path = CString::new("/dev/null").expect("dev null path contains NUL");
        let mode = CString::new("r").expect("dev null mode contains NUL");
        unsafe { libc::fopen(path.as_ptr(), mode.as_ptr()) as usize }
    });
    *stream as *mut libc::FILE
}

fn pump(fd: c_int, chunk_tx: UnboundedSender<Vec<u8>>) {
    let mut file = unsafe { std::fs::File::from_raw_fd(fd) };
    let mut buf = [0u8; 8192];
    loop {
        let Ok(read) = file.read(&mut buf) else {
            break;
        };
        if read == 0 || chunk_tx.send(buf[..read].to_vec()).is_err() {
            break;
        }
    }
}
