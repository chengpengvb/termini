use russh::client::Handle;
use crate::common::context::context_ssh::ContextSsh;
use crate::model::resp::event::resp_event_ssh_auth::SshOpenCallback;

pub struct SshAuthCall<'a, H: russh::client::Handler> {
    pub username: &'a str,
    pub can_reselect: bool,
    pub channel_id: &'a str,
    pub context: &'a ContextSsh,
    pub handle: &'a mut Handle<H>,
    pub handler: &'a SshOpenCallback,
}

impl<'a, H: russh::client::Handler> SshAuthCall<'a, H> {
    pub fn new(channel_id: &'a str, handle: &'a mut Handle<H>, username: &'a str, context: &'a ContextSsh, handler: &'a SshOpenCallback, can_reselect: bool) -> Self {
        Self { channel_id, handle, username, context, handler, can_reselect }
    }
}
