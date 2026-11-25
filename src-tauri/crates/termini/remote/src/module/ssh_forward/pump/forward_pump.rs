use russh::Channel;
use russh::client::Msg;
use tokio::net::TcpStream;

pub async fn pump(channel: Channel<Msg>, mut tcp: TcpStream) {
    let mut stream = channel.into_stream();
    let _ = tokio::io::copy_bidirectional(&mut stream, &mut tcp).await;
}
