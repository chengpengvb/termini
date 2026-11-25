use std::path::PathBuf;
use tokio::task::AbortHandle;

pub struct FileEditSession {
    pub local_dir: PathBuf,
    pub abort_handle: AbortHandle,
    pub _watcher: notify::RecommendedWatcher,
}
