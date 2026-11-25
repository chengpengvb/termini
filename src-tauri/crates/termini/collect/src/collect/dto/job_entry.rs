use sacs::task::TaskId;
use crate::collect::dto::JobModel;

pub struct JobEntry {
    pub model: JobModel,
    pub task_id: Option<TaskId>,
}
