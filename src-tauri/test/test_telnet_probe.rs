use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::module::telnet::core::probe::probe_telnet;
use crate::module::telnet::core::command::{TELNET_IAC, TELNET_TELOPT_ECHO, TELNET_TELOPT_SGA, TELNET_WILL};

async fn connect_to_mock<F, Fut>(handler: F) -> TcpStream
where
    F: FnOnce(TcpStream) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind mock telnet");
    let addr = listener.local_addr().expect("local addr");
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        handler(stream).await;
    });
    TcpStream::connect(addr).await.expect("connect mock")
}

async fn drain_probe_negotiation(server: &mut TcpStream) {
    let mut buffer = [0u8; 64];
    let _ = server.read(&mut buffer).await;
}

#[tokio::test]
async fn probe_completes_without_note_when_negotiation_arrives_in_first_packet() {
    let client = connect_to_mock(|mut server| async move {
        drain_probe_negotiation(&mut server).await;
        let reply = [TELNET_IAC, TELNET_WILL, TELNET_TELOPT_ECHO, TELNET_IAC, TELNET_WILL, TELNET_TELOPT_SGA];
        server.write_all(&reply).await.expect("write iac");
    })
    .await;

    let result = probe_telnet(client).await.expect("首包含协商应探测成功");
    assert!(result.note.is_none(), "拿到协商证据时不应带说明: {:?}", result.note);
}

#[tokio::test]
async fn probe_completes_without_note_when_negotiation_arrives_after_banner() {
    let client = connect_to_mock(|mut server| async move {
        drain_probe_negotiation(&mut server).await;
        server.write_all(b"\r\nPress RETURN to get started\r\n").await.expect("banner");
        tokio::time::sleep(Duration::from_millis(30)).await;
        let reply = [TELNET_IAC, TELNET_WILL, TELNET_TELOPT_ECHO];
        server.write_all(&reply).await.expect("delayed iac");
    })
    .await;

    let result = probe_telnet(client).await.expect("协商晚一拍到达仍应成功");
    assert!(result.note.is_none(), "窗口内拿到协商证据时不应带说明: {:?}", result.note);
    assert!(result.initial_data.starts_with(b"\r\nPress RETURN"), "先到的横幅必须留给 codec 重放");
}

#[tokio::test]
async fn probe_completes_with_note_when_only_banner_arrives() {
    let client = connect_to_mock(|mut server| async move {
        drain_probe_negotiation(&mut server).await;
        server.write_all(b"\r\nUser Access Verification\r\n\r\nUsername: ").await.expect("banner");
        tokio::time::sleep(Duration::from_secs(3)).await;
    })
    .await;

    let result = probe_telnet(client).await.expect("纯文本横幅应放行而不是判失败");
    assert!(result.note.is_some(), "未见协商时应带说明");
    assert_eq!(result.initial_data, b"\r\nUser Access Verification\r\n\r\nUsername: ".to_vec(), "横幅必须完整留给 codec 重放");
}

#[tokio::test]
async fn probe_completes_with_note_when_peer_stays_silent() {
    let client = connect_to_mock(|mut server| async move {
        drain_probe_negotiation(&mut server).await;
        tokio::time::sleep(Duration::from_secs(3)).await;
    })
    .await;

    let result = probe_telnet(client).await.expect("对端不吭声应放行，交给后续阶段兜底");
    assert!(result.note.is_some(), "窗口内无输出时应带说明");
    assert!(result.initial_data.is_empty(), "没有数据时不应凭空造出初始字节");
}

#[tokio::test]
async fn probe_fails_when_peer_is_not_telnet_service() {
    let client = connect_to_mock(|mut server| async move {
        drain_probe_negotiation(&mut server).await;
        server.write_all(b"SSH-2.0-OpenSSH_9.6\r\n").await.expect("ssh banner");
        tokio::time::sleep(Duration::from_secs(3)).await;
    })
    .await;

    match probe_telnet(client).await {
        Ok(_) => panic!("SSH 端口不应被当成 telnet"),
        Err(error) => assert!(format!("{error:#}").contains("not a telnet service"), "实际: {error:#}"),
    }
}

#[tokio::test]
async fn probe_fails_when_peer_closes_before_negotiation() {
    let client = connect_to_mock(|mut server| async move {
        drain_probe_negotiation(&mut server).await;
        server.shutdown().await.expect("close");
    })
    .await;

    match probe_telnet(client).await {
        Ok(_) => panic!("对端直接关闭应判失败"),
        Err(error) => assert!(format!("{error:#}").contains("closed before telnet negotiation"), "实际: {error:#}"),
    }
}
