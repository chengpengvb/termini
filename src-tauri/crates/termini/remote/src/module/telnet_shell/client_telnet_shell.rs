use std::sync::Arc;
use tokio::net::TcpStream;
use anyhow::{anyhow, Result};
use std::time::{Duration, Instant};
use summer::tracing::{debug, warn};
use tokio::net::tcp::OwnedReadHalf;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::sync::Mutex as AsyncMutex;
use crate::model::device::DeviceOsType;
use tokio::time::timeout as await_timeout;
use crate::module::telnet::core::command::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::module::telnet::prompt::prompt_text;
use crate::module::telnet::core::codec::TelnetCodec;
use crate::module::telnet::core::event::TelnetEvent;
use crate::util::ssh_shell::shell_text::decode_shell_bytes;
use crate::module::telnet::core::environ::encode_new_environ;
use crate::module::telnet::prompt::prompt_pattern::TelnetPromptPattern;
use crate::service::service_device_type::resolve_disable_paging_command;
use crate::module::common::dto::shell_chunk_callback::ShellChunkCallback;
use crate::module::telnet::core::subnegotiation::encode_terminal_type_is;
use crate::module::telnet::core::probe::{probe_telnet, TelnetProbeResult};
use crate::module::common::context::context_telnet_shell::ContextTelnetShell;
use crate::module::telnet::gate::telnet_session_gate::{TelnetPermit, TelnetSessionGate, TELNET_GATE_WAIT};

const TERMINAL_COLS: u16 = 511;
const TERMINAL_ROWS: u16 = 1000;

const READ_BUFFER_SIZE: usize = 8192;
const DEFAULT_TERMINAL_TYPE: &str = "vt100";

const SETTLE_ROUNDS: u32 = 6;
const MATCH_WINDOW: usize = 4096;
const MAX_LOGIN_NUDGES: u32 = 6;
const MAX_CREDENTIAL_SENDS: u32 = 5;

const READ_POLL: Duration = Duration::from_millis(50);
const TELNET_SHELL_IDLE_TIMEOUT: Duration = Duration::from_secs(15);
pub const TELNET_SHELL_LOGIN_TIMEOUT: Duration = Duration::from_secs(60);

struct TelnetShellSession {
    permit: TelnetPermit,
    buffer: String, //当轮命令的原始回显累积；ANSI 剥离放在匹配时做，避免分片切断转义序列
    codec: TelnetCodec,
    prompt_line: String, //每条命令前探针读到的提示符原文，如 "<Switch>"
    terminal_type: String,
    reader: OwnedReadHalf,
    writer: OwnedWriteHalf,
    pattern: TelnetPromptPattern,
    cmd_verify: bool,             //是否做命令回显校验（同 Netmiko cmd_verify），关掉即跳过等回显这一步
    cmd_verify_timeout: Duration, //回显校验的独立超时，超过即报错，提示用户关掉 cmd_verify
}

pub struct ClientTelnetShell {
    session_id: String,
    state: Arc<AsyncMutex<Option<TelnetShellSession>>>,
}

impl ClientTelnetShell {
    pub fn new(session_id: String) -> Self {
        Self { session_id, state: Arc::new(AsyncMutex::new(None)) }
    }

    pub async fn open(&self, context: &ContextTelnetShell) -> Result<()> {
        let host = context.device.read().host.clone();
        let (port, limit) = {
            let config = context.config.read();
            (config.get_base()?.port, config.resolve_max_sessions())
        };
        let permit = TelnetSessionGate::global().acquire(&host, port, limit, TELNET_GATE_WAIT).await?;
        self.open_with_permit(context, permit).await
    }

    pub async fn open_with_permit(&self, context: &ContextTelnetShell, permit: TelnetPermit) -> Result<()> {
        let mut guard = self.state.lock().await;
        if guard.is_some() {
            return Err(anyhow!("telnet shell already opened"));
        }
        let host = context.device.read().host.clone();
        let base = context.config.read().get_base()?;
        let stream = Self::connect_tcp(&host, base.port, base.connect_timeout_seconds).await?;
        let TelnetProbeResult { stream, codec, initial_data, .. } = probe_telnet(stream).await.map_err(|error| anyhow!("Telnet probe failed: {}", error))?;

        let (nvt_eol, cmd_verify, cmd_verify_timeout) = {
            let config = context.config.read();
            (config.nvt_eol, config.resolve_cmd_verify(), Duration::from_secs(config.resolve_cmd_verify_timeout_seconds() as u64))
        };
        let pattern = TelnetPromptPattern::from_config(&context.config.read());
        let terminal_type = Self::resolve_terminal_type(context);
        let (reader, writer) = stream.into_split();
        let mut session = TelnetShellSession { codec, reader, writer, pattern, terminal_type, permit, buffer: String::new(), prompt_line: String::new(), cmd_verify, cmd_verify_timeout };
        if nvt_eol {
            let flags = session.codec.flags() | TELNET_FLAG_NVT_EOL;
            session.codec.set_flags(flags);
        }
        Self::negotiate(&mut session).await?;
        Self::consume_events(&mut session, initial_data.clone()).await?;

        let deadline = Instant::now() + TELNET_SHELL_LOGIN_TIMEOUT;
        Self::login(&mut session, base.username.clone(), base.password.clone(), deadline).await?;
        debug!("[telnet] Telnet Shell 打开 [{}:{}] 成功. session={}", host, base.port, self.session_id);

        let os_type = DeviceOsType::from_i16(context.device.read().os_type);
        let paging_command = Self::resolve_disable_paging(context, os_type);
        *guard = Some(session);
        drop(guard);
        if let Some(command) = paging_command {
            self.disable_paging(&command).await;
        }
        Ok(())
    }

    pub async fn close(&self) -> Result<()> {
        let Some(mut session) = self.state.lock().await.take() else {
            return Ok(());
        };
        let _ = session.writer.shutdown().await;
        debug!("[telnet] Telnet Shell 关闭 [{}]. session={}", session.permit.key(), self.session_id);
        Ok(())
    }

    pub async fn exec(&self, command: String, exec_timeout: Duration, on_chunk: Option<ShellChunkCallback>) -> Result<String> {
        let mut guard = self.state.lock().await;
        let session = guard.as_mut().ok_or_else(|| anyhow!("telnet shell not opened"))?;
        let lines: Vec<&str> = command.lines().map(|line| line.trim_end()).filter(|line| !line.trim().is_empty()).collect();
        if lines.is_empty() {
            return Err(anyhow!("command is empty"));
        }
        let deadline = Instant::now() + exec_timeout;
        let mut outputs: Vec<String> = Vec::with_capacity(lines.len());
        for line in lines {
            debug!("[telnet] 行开始 cmd={:?} prompt={}", line, session.prompt_line);
            let output = Self::exec_line(session, line, deadline, on_chunk.as_ref()).await?;
            debug!("[telnet] 行结束 cmd={:?} prompt={}", line, session.prompt_line);
            if !output.is_empty() {
                outputs.push(output);
            }
        }
        Ok(outputs.join("\n"))
    }

    async fn disable_paging(&self, command: &str) {
        let mut guard = self.state.lock().await;
        let Some(session) = guard.as_mut() else {
            return;
        };
        let deadline = Instant::now() + TELNET_SHELL_LOGIN_TIMEOUT;
        if let Err(error) = Self::exec_line(session, command, deadline, None).await {
            warn!("[telnet] 关闭分页失败 session={} command={} error={:#}", self.session_id, command, error);
        }
    }

    async fn exec_line(session: &mut TelnetShellSession, line: &str, deadline: Instant, on_chunk: Option<&ShellChunkCallback>) -> Result<String> {
        session.prompt_line = Self::find_prompt(session, deadline).await?;
        session.buffer.clear();
        Self::write_text(session, &prompt_text::normalize_command(line)).await?;
        if session.cmd_verify {
            let echo_deadline = (Instant::now() + session.cmd_verify_timeout).min(deadline);
            Self::read_until_command_echo(session, line, echo_deadline, on_chunk).await?;
        }
        let ended = Self::read_until_prompt(session, line, deadline, on_chunk).await?;
        let text = prompt_text::normalize_linefeeds(&prompt_text::strip_ansi_escape_codes(&session.buffer));
        let text = prompt_text::strip_command_echo(line, &text);
        let text = prompt_text::strip_trailing_prompt(&ended, &text);
        let output = prompt_text::trim_line_ends(&text);
        Ok(output.trim_end().to_string())
    }

    async fn read_until_command_echo(session: &mut TelnetShellSession, line: &str, deadline: Instant, on_chunk: Option<&ShellChunkCallback>) -> Result<()> {
        let command = line.trim();
        if command.is_empty() {
            return Ok(());
        }
        let mut last_data = Instant::now();
        loop {
            if prompt_text::strip_backspaces(&Self::tail_view(&session.buffer)).contains(command) {
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err(anyhow!("command {:?} echo verification timed out, command echo not seen (disable cmd_verify or increase its timeout in the device Telnet config)", line));
            }
            let delta = Self::read_channel(session).await?;
            if !delta.is_empty() {
                last_data = Instant::now();
                if let Some(callback) = on_chunk {
                    callback(delta.clone().into_bytes()).await;
                }
            } else if last_data.elapsed() >= TELNET_SHELL_IDLE_TIMEOUT {
                return Err(anyhow!("command {:?} silent for {}s without command echo, channel out of sync", line, TELNET_SHELL_IDLE_TIMEOUT.as_secs()));
            }
            Self::continue_paging(session).await?;
        }
    }

    async fn read_until_prompt(session: &mut TelnetShellSession, line: &str, deadline: Instant, on_chunk: Option<&ShellChunkCallback>) -> Result<String> {
        let mut last_data = Instant::now();
        loop {
            if Instant::now() >= deadline {
                return Err(anyhow!("command {:?} exceeded total execution timeout, prompt {} not matched", line, session.prompt_line));
            }
            let delta = Self::read_channel(session).await?;
            if !delta.is_empty() {
                last_data = Instant::now();
                if let Some(callback) = on_chunk {
                    callback(delta.clone().into_bytes()).await;
                }
                let tail = Self::tail_view(&session.buffer);
                let matched = session.pattern.is_command_end(&session.prompt_line, &tail);
                debug!("[telnet] 收到数据 matched={} expect={} tail={:?} delta={:?}", matched, session.prompt_line, tail, delta);
            } else if last_data.elapsed() >= TELNET_SHELL_IDLE_TIMEOUT {
                return Err(anyhow!("command {:?} silent for {}s without prompt {}, treated as hung", line, TELNET_SHELL_IDLE_TIMEOUT.as_secs(), session.prompt_line));
            }
            if Self::continue_paging(session).await? {
                continue;
            }
            let tail = Self::tail_view(&session.buffer);
            if !session.pattern.is_command_end(&session.prompt_line, &tail) {
                continue;
            }
            let Some(ended) = prompt_text::extract_prompt_line(&tail) else {
                return Err(anyhow!("command {:?} matched the end condition but no prompt line could be extracted: {:?}", line, tail));
            };
            return Ok(ended);
        }
    }

    async fn continue_paging(session: &mut TelnetShellSession) -> Result<bool> {
        let start = Self::window_start(&session.buffer);
        let Some((matched_start, matched_end)) = session.pattern.find_more_prompt(&session.buffer[start..]) else {
            return Ok(false);
        };
        let from = start + matched_start;
        let mut to = start + matched_end;
        while let Some(next) = session.buffer[to..].chars().next() {
            if matches!(next, '\u{8}' | ' ' | '\r') {
                to += next.len_utf8();
                continue;
            }
            break;
        }
        session.buffer.replace_range(from..to, "");
        let response = session.pattern.more_response().to_string();
        Self::write_text(session, &response).await?;
        Ok(true)
    }

    async fn login(session: &mut TelnetShellSession, username: Option<String>, password: Option<String>, deadline: Instant) -> Result<()> {
        let mut nudges = 0u32;
        let mut idle_rounds = 0u32;
        let mut username_sends = 0u32;
        let mut password_sends = 0u32;
        loop {
            if Instant::now() >= deadline {
                return Err(anyhow!("telnet login timed out, device prompt not matched"));
            }
            let delta = Self::read_channel(session).await?;
            if delta.is_empty() {
                idle_rounds += 1;
            } else {
                idle_rounds = 0;
            }
            let view = Self::tail_view(&session.buffer);
            if let Some(value) = Self::pick_credential(&username, username_sends, session.pattern.is_username_prompt(&view)) {
                Self::send_credential(session, &value).await?;
                username_sends += 1;
                continue;
            }
            if let Some(value) = Self::pick_credential(&password, password_sends, session.pattern.is_password_prompt(&view)) {
                Self::send_credential(session, &value).await?;
                password_sends += 1;
                continue;
            }
            let credential_sent = username_sends > 0 || password_sends > 0;
            if session.pattern.is_device_prompt(&view) && (credential_sent || idle_rounds >= SETTLE_ROUNDS) {
                return Ok(());
            }
            if idle_rounds < SETTLE_ROUNDS {
                continue;
            }
            if nudges >= MAX_LOGIN_NUDGES {
                return Err(anyhow!("telnet login failed: device neither asked for credentials nor showed a recognizable prompt"));
            }
            Self::write_text(session, prompt_text::COMMAND_EOL).await?;
            nudges += 1;
            idle_rounds = 0;
        }
    }

    fn pick_credential(value: &Option<String>, sends: u32, matched: bool) -> Option<String> {
        if !matched || sends >= MAX_CREDENTIAL_SENDS {
            return None;
        }
        let text = value.as_deref().map(|text| text.trim()).filter(|text| !text.is_empty())?;
        Some(text.to_string())
    }

    async fn send_credential(session: &mut TelnetShellSession, value: &str) -> Result<()> {
        Self::write_text(session, &prompt_text::normalize_command(value)).await?;
        session.buffer.clear();
        Ok(())
    }

    async fn find_prompt(session: &mut TelnetShellSession, deadline: Instant) -> Result<String> {
        session.buffer.clear();
        Self::write_text(session, prompt_text::COMMAND_EOL).await?;
        let mut last_data = Instant::now();
        loop {
            if Instant::now() >= deadline {
                return Err(anyhow!("timed out confirming device prompt; adjust the prompt regex in the device Telnet config"));
            }
            let delta = Self::read_channel(session).await?;
            if delta.is_empty() {
                if last_data.elapsed() >= TELNET_SHELL_IDLE_TIMEOUT {
                    return Err(anyhow!("device silent for {}s without echoing a prompt; adjust the prompt regex in the device Telnet config", TELNET_SHELL_IDLE_TIMEOUT.as_secs()));
                }
                continue;
            }
            last_data = Instant::now();
            let view = Self::tail_view(&session.buffer);
            if !session.pattern.is_device_prompt(&view) {
                continue;
            }
            let Some(line) = prompt_text::extract_prompt_line(&view) else {
                return Err(anyhow!("prompt regex matched but no prompt line could be extracted: {:?}", view));
            };
            return Ok(line);
        }
    }

    async fn read_channel(session: &mut TelnetShellSession) -> Result<String> {
        let mut buf = [0u8; READ_BUFFER_SIZE];
        let size = match await_timeout(READ_POLL, session.reader.read(&mut buf)).await {
            Err(_) => return Ok(String::new()),
            Ok(Ok(0)) => return Err(anyhow!("telnet connection closed by peer")),
            Ok(Ok(size)) => size,
            Ok(Err(error)) => return Err(anyhow!("failed to read telnet data: {}", error)),
        };
        let chunk = buf[..size].to_vec();
        Self::consume_events(session, chunk).await
    }

    async fn consume_events(session: &mut TelnetShellSession, chunk: Vec<u8>) -> Result<String> {
        if chunk.is_empty() {
            return Ok(String::new());
        }
        let events = session.codec.receive(&chunk);
        let mut delta = String::new();
        for event in events {
            match event {
                TelnetEvent::Data(bytes) => {
                    delta.push_str(&decode_shell_bytes(&bytes));
                }
                TelnetEvent::Send(bytes) => {
                    session.writer.write_all(&bytes).await.map_err(|error| anyhow!("failed to write telnet negotiation response: {}", error))?;
                }
                TelnetEvent::TerminalType { command, .. } if command == TELNET_TTYPE_SEND => {
                    let bytes = encode_terminal_type_is(session.terminal_type.as_bytes());
                    session.writer.write_all(&bytes).await.map_err(|error| anyhow!("failed to write telnet terminal type: {}", error))?;
                }
                TelnetEvent::Environ { command, .. } if command == TELNET_ENVIRON_SEND => {
                    let bytes = encode_new_environ(TELNET_ENVIRON_IS, &[]);
                    session.writer.write_all(&bytes).await.map_err(|error| anyhow!("failed to write telnet environment variables: {}", error))?;
                }
                TelnetEvent::Error(error) => {
                    return Err(anyhow!("telnet protocol error: {}", error.message));
                }
                _ => {}
            }
        }
        session.writer.flush().await.map_err(|error| anyhow!("failed to flush telnet connection: {}", error))?;
        session.buffer.push_str(&delta);
        Ok(delta)
    }

    async fn write_text(session: &mut TelnetShellSession, text: &str) -> Result<()> {
        let event = session.codec.send_text(text.as_bytes());
        match event {
            TelnetEvent::Send(bytes) => {
                session.writer.write_all(&bytes).await.map_err(|error| anyhow!("failed to write telnet data: {}", error))?;
                session.writer.flush().await.map_err(|error| anyhow!("failed to flush telnet connection: {}", error))?;
                Ok(())
            }
            TelnetEvent::Error(error) => Err(anyhow!("{}", error.message)),
            _ => Ok(()),
        }
    }

    async fn negotiate(session: &mut TelnetShellSession) -> Result<()> {
        let mut events = session.codec.negotiate(TELNET_DO, TELNET_TELOPT_EOR);
        let body = Self::encode_naws_body(TERMINAL_COLS, TERMINAL_ROWS);
        events.extend(session.codec.subnegotiation(TELNET_TELOPT_NAWS, &body));
        for event in events {
            if let TelnetEvent::Send(bytes) = event {
                session.writer.write_all(&bytes).await.map_err(|error| anyhow!("failed to write telnet negotiation: {}", error))?;
            }
        }
        session.writer.flush().await.map_err(|error| anyhow!("failed to flush telnet connection: {}", error))?;
        Ok(())
    }

    fn encode_naws_body(cols: u16, rows: u16) -> [u8; 4] {
        let cols = cols.to_be_bytes();
        let rows = rows.to_be_bytes();
        [cols[0], cols[1], rows[0], rows[1]]
    }

    fn window_start(buffer: &str) -> usize {
        let start = buffer.len().saturating_sub(MATCH_WINDOW);
        (start..buffer.len()).find(|index| buffer.is_char_boundary(*index)).unwrap_or(0)
    }

    fn tail_view(buffer: &str) -> String {
        let start = Self::window_start(buffer);
        prompt_text::normalize_linefeeds(&prompt_text::strip_ansi_escape_codes(&buffer[start..]))
    }

    fn resolve_terminal_type(context: &ContextTelnetShell) -> String {
        let config = context.config.read();
        if let Ok(Some(types)) = config.get_terminal_types() {
            if let Some(name) = types.into_iter().find(|name| !name.is_empty()) {
                return name;
            }
        }
        match config.get_base() {
            Ok(base) if !base.term.is_empty() => base.term,
            _ => DEFAULT_TERMINAL_TYPE.to_string(),
        }
    }

    fn resolve_disable_paging(context: &ContextTelnetShell, os_type: DeviceOsType) -> Option<String> {
        let configured = context.config.read().disable_paging_command.clone();
        if let Some(command) = configured.map(|text| text.trim().to_string()) {
            if !command.is_empty() {
                return Some(command);
            }
            return None;
        }
        resolve_disable_paging_command(os_type).map(|command| command.to_string())
    }

    async fn connect_tcp(host: &str, port: u16, connect_timeout_seconds: Option<u64>) -> Result<TcpStream> {
        let seconds = connect_timeout_seconds.unwrap_or(3).clamp(3, 120);
        match await_timeout(Duration::from_secs(seconds), TcpStream::connect((host, port))).await {
            Ok(Ok(stream)) => Ok(stream),
            Ok(Err(error)) => Err(anyhow!("Failed to connect telnet {}:{}: {}", host, port, error)),
            Err(_) => Err(anyhow!("Failed to connect telnet {}:{}: connection timed out ({}s)", host, port, seconds)),
        }
    }
}
