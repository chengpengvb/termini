use tokio::net::TcpStream;
use crate::codec::TelnetCodec;
use crate::event::TelnetEvent;
use anyhow::{anyhow, Context, Result};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::{timeout, Duration, Instant};
use crate::command::{TELNET_DO, TELNET_DONT, TELNET_IAC, TELNET_TELOPT_BINARY, TELNET_TELOPT_ECHO, TELNET_TELOPT_NAWS, TELNET_TELOPT_SGA, TELNET_TELOPT_TTYPE, TELNET_WILL, TELNET_WONT};

const TELNET_PROBE_BUFFER_SIZE: usize = 1024;
const TELNET_PROBE_WINDOW: Duration = Duration::from_millis(1500);

pub struct TelnetProbeResult {
    pub stream: TcpStream,
    pub codec: TelnetCodec,
    pub initial_data: Vec<u8>,
    pub note: Option<String>, //未见协商证据时的说明，交给上层作为探测步骤的附带信息
}

pub async fn probe_telnet(mut stream: TcpStream) -> Result<TelnetProbeResult> {
    let mut codec = TelnetCodec::default();
    send_probe_negotiation(&mut stream, &mut codec).await?;
    let initial_data = read_probe_window(&mut stream).await?;
    let note = classify_probe_response(&initial_data)?;
    Ok(TelnetProbeResult { stream, codec, initial_data, note })
}

async fn send_probe_negotiation(stream: &mut TcpStream, codec: &mut TelnetCodec) -> Result<()> {
    let mut events = Vec::new();
    events.extend(codec.negotiate(TELNET_WILL, TELNET_TELOPT_NAWS));
    events.extend(codec.negotiate(TELNET_WILL, TELNET_TELOPT_TTYPE));
    events.extend(codec.negotiate(TELNET_DO, TELNET_TELOPT_ECHO));
    events.extend(codec.negotiate(TELNET_DO, TELNET_TELOPT_SGA));
    for event in events {
        write_probe_event(stream, event).await?;
    }
    Ok(())
}

async fn write_probe_event(stream: &mut TcpStream, event: TelnetEvent) -> Result<()> {
    if let TelnetEvent::Send(data) = event {
        stream.write_all(&data).await.context("send telnet probe negotiation")?;
    }
    Ok(())
}

//设备常常先吐横幅、协商回应晚一拍才到，所以读满窗口而不是只看首包，读到的字节全部留给 codec 重放
async fn read_probe_window(stream: &mut TcpStream) -> Result<Vec<u8>> {
    let deadline = Instant::now() + TELNET_PROBE_WINDOW;
    let mut collected = Vec::new();
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Ok(collected);
        }
        let mut buffer = [0; TELNET_PROBE_BUFFER_SIZE];
        let Ok(read_result) = timeout(remaining, stream.read(&mut buffer)).await else {
            return Ok(collected);
        };
        let size = read_result.context("read telnet probe response")?;
        if size == 0 {
            return Err(anyhow!("remote endpoint closed before telnet negotiation"));
        }
        collected.extend_from_slice(&buffer[..size]);
        if has_telnet_negotiation_response(&collected) {
            return Ok(collected);
        }
    }
}

//只有对端明显不是 telnet 才判失败，没有协商证据时按 NVT 文本继续，与 libtelnet 的处理一致
fn classify_probe_response(data: &[u8]) -> Result<Option<String>> {
    if has_telnet_negotiation_response(data) {
        return Ok(None);
    }
    if is_non_telnet_banner(data) {
        return Err(anyhow!("remote endpoint is not a telnet service"));
    }
    if data.is_empty() {
        return Ok(Some("probe: no output from peer yet, continuing as NVT".to_string()));
    }
    Ok(Some("probe: no negotiation response seen, continuing as NVT text".to_string()))
}

fn has_telnet_negotiation_response(data: &[u8]) -> bool {
    let mut index = 0;
    while index + 2 < data.len() {
        if data[index] == TELNET_IAC && is_valid_negotiation_response(data[index + 1], data[index + 2]) {
            return true;
        }
        index += 1;
    }
    false
}

fn is_valid_negotiation_response(command: u8, telopt: u8) -> bool {
    match telopt {
        TELNET_TELOPT_NAWS | TELNET_TELOPT_TTYPE => command == TELNET_DO || command == TELNET_DONT,
        TELNET_TELOPT_ECHO | TELNET_TELOPT_SGA => command == TELNET_WILL || command == TELNET_WONT,
        TELNET_TELOPT_BINARY => command == TELNET_DO || command == TELNET_DONT || command == TELNET_WILL || command == TELNET_WONT,
        _ => false,
    }
}

fn is_non_telnet_banner(data: &[u8]) -> bool {
    let text = match std::str::from_utf8(data) {
        Ok(value) => value.trim_start().to_ascii_uppercase(),
        Err(_) => return true,
    };
    text.starts_with("SSH-") || text.starts_with("HTTP/") || text.starts_with("FTP") || text.starts_with("220 ") || text.starts_with("SMTP")
}
