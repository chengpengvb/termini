use rig::completion::Message;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryTurn {
    pub item_id: String,
    pub messages: Vec<Message>,
}

impl MemoryTurn {
    pub fn flatten(turns: &[MemoryTurn]) -> Vec<Message> {
        turns.iter().flat_map(|turn| turn.messages.iter().cloned()).collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatScene {
    pub task_kind: i16,

    #[serde(default)]
    pub goal: Option<String>,

    #[serde(default)]
    pub language: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LoadedMemory {
    pub summary: String,
    pub turns: Vec<MemoryTurn>,
    pub scene: Option<ChatScene>,
}

impl LoadedMemory {
    pub fn messages(&self) -> Vec<Message> {
        MemoryTurn::flatten(&self.turns)
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryPayload {
    #[serde(default)]
    pub summary: String,

    pub turns: Vec<MemoryTurn>,

    #[serde(default)]
    pub scene: Option<ChatScene>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryPayloadFlat {
    #[serde(default)]
    pub summary: String,

    pub messages: Vec<Message>,
}
