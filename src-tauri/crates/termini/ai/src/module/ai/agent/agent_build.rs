use anyhow::Result;
use futures::StreamExt;
use rig::agent::StreamingError;
use rig::agent::MultiTurnStreamItem;
use crate::model::ai_provide::AiProvide;
use rig::tool::server::ToolServerHandle;
use rig::tool::{DynamicTool, ToolContext};
use crate::module::ai::provide::ai_agent::AiAgent;
use crate::module::ai::provide::provide_factory::ProvideFactory;
use rig::streaming::{StreamedAssistantContent, StreamingPrompt};
use rig::completion::{CompletionError, Message, Prompt, PromptError};

const LIGHT_TOOL_CONCURRENCY: usize = 4;

#[derive(Debug, Clone, Copy)]
pub struct AgentRunSpec {
    pub max_turns: usize,
    pub tool_concurrency: usize,
}

impl AgentRunSpec {
    pub fn new(max_turns: usize, tool_concurrency: usize) -> Self {
        Self { max_turns, tool_concurrency }
    }

    pub fn light(max_turns: usize) -> Self {
        Self { max_turns, tool_concurrency: LIGHT_TOOL_CONCURRENCY }
    }
}

pub fn build_agent(provide_factory: &ProvideFactory, ai_provide: &AiProvide, preamble: &str, tools: Vec<DynamicTool>) -> Result<AiAgent> {
    provide_factory.build_agent_with_thinking(ai_provide, preamble, tools)
}

pub fn build_agent_no_thinking(provide_factory: &ProvideFactory, ai_provide: &AiProvide, preamble: &str, tools: Vec<DynamicTool>) -> Result<AiAgent> {
    provide_factory.build_agent_without_thinking(ai_provide, preamble, tools)
}

pub fn build_agent_with_level(provide_factory: &ProvideFactory, ai_provide: &AiProvide, preamble: &str, tools: Vec<DynamicTool>, level: Option<&str>) -> Result<AiAgent> {
    provide_factory.build_agent_with_level(ai_provide, preamble, tools, level)
}

pub fn build_agent_with_level_handle(provide_factory: &ProvideFactory, ai_provide: &AiProvide, preamble: &str, handle: ToolServerHandle, level: Option<&str>) -> Result<AiAgent> {
    provide_factory.build_agent_with_level_handle(ai_provide, preamble, handle, level)
}

pub async fn prompt_agent(agent: AiAgent, prompt: impl Into<Message>) -> Result<String> {
    let prompt = prompt.into();
    Ok(agent.prompt(prompt).await?)
}

pub fn is_transport_completion(error: &CompletionError) -> bool {
    match error {
        CompletionError::HttpError(_) => true,
        CompletionError::ProviderResponse(_) => true,
        CompletionError::ProviderError(message) => message.starts_with("Http client error:"),
        _ => false,
    }
}

pub fn is_transport_streaming(error: &StreamingError) -> bool {
    match error {
        StreamingError::Completion(completion) => is_transport_completion(completion),
        StreamingError::Prompt(prompt) => match prompt.as_ref() {
            PromptError::CompletionError(completion) => is_transport_completion(completion),
            _ => false,
        },
    }
}

pub fn is_transport_error(error: &anyhow::Error) -> bool {
    if let Some(streaming) = error.downcast_ref::<StreamingError>() {
        return is_transport_streaming(streaming);
    }
    match error.downcast_ref::<PromptError>() {
        Some(PromptError::CompletionError(completion)) => is_transport_completion(completion),
        Some(_) => false,
        None => match error.downcast_ref::<CompletionError>() {
            Some(completion) => is_transport_completion(completion),
            None => false,
        },
    }
}

pub async fn stream_prompt_text(agent: AiAgent, prompt: impl Into<Message>) -> Result<String> {
    stream_prompt_text_with(agent, prompt, |_| {}).await
}

pub async fn stream_prompt_text_with(agent: AiAgent, prompt: impl Into<Message>, mut on_delta: impl FnMut(&str)) -> Result<String> {
    let streamed = stream_prompt_text_until(agent, prompt, |delta| {
        on_delta(delta);
        StreamControl::Continue
    })
    .await?;
    Ok(streamed.text)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamControl {
    Continue,
    Stop,
}

pub struct StreamedText {
    pub text: String,
    pub stopped: bool,
}

pub async fn stream_prompt_text_until(agent: AiAgent, prompt: impl Into<Message>, mut on_delta: impl FnMut(&str) -> StreamControl) -> Result<StreamedText> {
    let prompt = prompt.into();
    let mut stream = agent.stream_prompt(prompt).max_turns(1).await;
    let mut output = String::new();
    let mut final_text = String::new();
    while let Some(item) = stream.next().await {
        match item? {
            MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::Text(text)) => {
                output.push_str(&text.text);
                if on_delta(&text.text) == StreamControl::Stop {
                    return Ok(StreamedText { text: output, stopped: true });
                }
            }
            MultiTurnStreamItem::FinalResponse(response) => final_text = response.output,
            _ => {}
        }
    }
    Ok(StreamedText { text: if final_text.trim().is_empty() { output } else { final_text }, stopped: false })
}

pub async fn prompt_agent_with_turns(agent: AiAgent, prompt: impl Into<Message>, max_turns: usize) -> Result<String> {
    prompt_agent_with_spec(agent, prompt, AgentRunSpec::light(max_turns), ToolContext::new()).await
}

pub async fn prompt_agent_with_extensions(agent: AiAgent, prompt: impl Into<Message>, max_turns: usize, tool_context: ToolContext) -> Result<String> {
    prompt_agent_with_spec(agent, prompt, AgentRunSpec::light(max_turns), tool_context).await
}

pub async fn prompt_agent_with_spec(agent: AiAgent, prompt: impl Into<Message>, spec: AgentRunSpec, tool_context: ToolContext) -> Result<String> {
    let prompt = prompt.into();
    Ok(agent.prompt(prompt).max_turns(spec.max_turns).tool_concurrency(spec.tool_concurrency).tool_context(tool_context).await?)
}

pub async fn probe_agent(provide_factory: &ProvideFactory, ai_provide: &AiProvide) -> Result<()> {
    let agent = build_agent(provide_factory, ai_provide, "", vec![])?;
    prompt_agent(agent, "ping").await?;
    Ok(())
}
