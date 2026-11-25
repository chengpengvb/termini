use russh::Channel;
use russh::client::Msg;

pub struct SshSession {
    pub channel_id: String,
    pub channel: Channel<Msg>,
}
