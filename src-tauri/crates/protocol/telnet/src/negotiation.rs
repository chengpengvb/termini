use crate::event::TelnetEvent;
use std::collections::HashMap;
use crate::error::{TelnetError, TelnetProtocolError};
use crate::option::{supports_him, supports_us, TelnetOptionSupport};
use crate::command::{TELNET_DO, TELNET_DONT, TELNET_FLAG_PROXY, TELNET_FLAG_RECEIVE_BINARY, TELNET_FLAG_TRANSMIT_BINARY, TELNET_IAC, TELNET_TELOPT_BINARY, TELNET_WILL, TELNET_WONT};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum QState {
    No,
    Yes,
    WantNo,
    WantYes,
    WantNoOpposite,
    WantYesOpposite,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct TelnetOptionState {
    pub us: QState,
    pub him: QState,
}

#[derive(Clone, Copy)]
enum Side {
    Us,
    Him,
}

impl Side {
    fn q(self, state: TelnetOptionState) -> QState {
        match self {
            Side::Us => state.us,
            Side::Him => state.him,
        }
    }
}

impl Default for TelnetOptionState {
    fn default() -> Self {
        Self { us: QState::No, him: QState::No }
    }
}

#[derive(Debug, Clone)]
pub struct TelnetNegotiator {
    telopts: Vec<TelnetOptionSupport>,
    states: HashMap<u8, TelnetOptionState>,
    flags: u8,
}

impl TelnetNegotiator {
    pub fn new(telopts: Vec<TelnetOptionSupport>, flags: u8) -> Self {
        Self { telopts, states: HashMap::new(), flags }
    }

    pub fn flags(&self) -> u8 {
        self.flags
    }

    pub fn set_flags(&mut self, flags: u8) {
        self.flags = flags;
    }

    pub fn is_proxy(&self) -> bool {
        self.flags & TELNET_FLAG_PROXY != 0
    }

    pub fn state(&self, telopt: u8) -> TelnetOptionState {
        match self.states.get(&telopt) {
            Some(state) => *state,
            None => TelnetOptionState::default(),
        }
    }

    pub fn receive(&mut self, command: u8, telopt: u8) -> Vec<TelnetEvent> {
        if self.is_proxy() {
            return Self::passthrough_event(command, telopt);
        }
        match command {
            TELNET_WILL => self.receive_will(telopt),
            TELNET_WONT => self.receive_wont(telopt),
            TELNET_DO => self.receive_do(telopt),
            TELNET_DONT => self.receive_dont(telopt),
            _ => Vec::new(),
        }
    }

    pub fn negotiate(&mut self, command: u8, telopt: u8) -> Vec<TelnetEvent> {
        if self.is_proxy() {
            return vec![Self::send_negotiate(command, telopt)];
        }
        match command {
            TELNET_WILL => self.negotiate_will(telopt),
            TELNET_WONT => self.negotiate_wont(telopt),
            TELNET_DO => self.negotiate_do(telopt),
            TELNET_DONT => self.negotiate_dont(telopt),
            _ => Vec::new(),
        }
    }

    fn passthrough_event(command: u8, telopt: u8) -> Vec<TelnetEvent> {
        match command {
            TELNET_WILL => vec![TelnetEvent::Will(telopt)],
            TELNET_WONT => vec![TelnetEvent::Wont(telopt)],
            TELNET_DO => vec![TelnetEvent::Do(telopt)],
            TELNET_DONT => vec![TelnetEvent::Dont(telopt)],
            _ => Vec::new(),
        }
    }

    fn receive_will(&mut self, telopt: u8) -> Vec<TelnetEvent> {
        let state = self.state(telopt);
        match state.him {
            QState::No => {
                if supports_him(&self.telopts, telopt) {
                    self.set_state(telopt, state.us, QState::Yes);
                    vec![Self::send_negotiate(TELNET_DO, telopt), TelnetEvent::Will(telopt)]
                } else {
                    vec![Self::send_negotiate(TELNET_DONT, telopt)]
                }
            }
            QState::WantNo => {
                self.set_state(telopt, state.us, QState::No);
                vec![Self::protocol_warning("DONT answered by WILL"), TelnetEvent::Wont(telopt)]
            }
            QState::WantNoOpposite => {
                self.set_state(telopt, state.us, QState::Yes);
                vec![Self::protocol_warning("DONT answered by WILL")]
            }
            QState::WantYes => {
                self.set_state(telopt, state.us, QState::Yes);
                vec![TelnetEvent::Will(telopt)]
            }
            QState::WantYesOpposite => {
                self.set_state(telopt, state.us, QState::WantNo);
                vec![Self::send_negotiate(TELNET_DONT, telopt), TelnetEvent::Will(telopt)]
            }
            QState::Yes => Vec::new(),
        }
    }

    fn receive_wont(&mut self, telopt: u8) -> Vec<TelnetEvent> {
        let state = self.state(telopt);
        match state.him {
            QState::Yes => {
                self.set_state(telopt, state.us, QState::No);
                vec![Self::send_negotiate(TELNET_DONT, telopt), TelnetEvent::Wont(telopt)]
            }
            QState::WantNo => {
                self.set_state(telopt, state.us, QState::No);
                vec![TelnetEvent::Wont(telopt)]
            }
            QState::WantNoOpposite => {
                self.set_state(telopt, state.us, QState::WantYes);
                vec![Self::send_negotiate(TELNET_DO, telopt), TelnetEvent::Wont(telopt)]
            }
            QState::WantYes | QState::WantYesOpposite => {
                self.set_state(telopt, state.us, QState::No);
                Vec::new()
            }
            QState::No => Vec::new(),
        }
    }

    fn receive_do(&mut self, telopt: u8) -> Vec<TelnetEvent> {
        let state = self.state(telopt);
        match state.us {
            QState::No => {
                if supports_us(&self.telopts, telopt) {
                    self.set_state(telopt, QState::Yes, state.him);
                    vec![Self::send_negotiate(TELNET_WILL, telopt), TelnetEvent::Do(telopt)]
                } else {
                    vec![Self::send_negotiate(TELNET_WONT, telopt)]
                }
            }
            QState::WantNo => {
                self.set_state(telopt, QState::No, state.him);
                vec![Self::protocol_warning("WONT answered by DO"), TelnetEvent::Dont(telopt)]
            }
            QState::WantNoOpposite => {
                self.set_state(telopt, QState::Yes, state.him);
                vec![Self::protocol_warning("WONT answered by DO")]
            }
            QState::WantYes => {
                self.set_state(telopt, QState::Yes, state.him);
                vec![TelnetEvent::Do(telopt)]
            }
            QState::WantYesOpposite => {
                self.set_state(telopt, QState::WantNo, state.him);
                vec![Self::send_negotiate(TELNET_WONT, telopt), TelnetEvent::Do(telopt)]
            }
            QState::Yes => Vec::new(),
        }
    }

    fn receive_dont(&mut self, telopt: u8) -> Vec<TelnetEvent> {
        let state = self.state(telopt);
        match state.us {
            QState::Yes => {
                self.set_state(telopt, QState::No, state.him);
                vec![Self::send_negotiate(TELNET_WONT, telopt), TelnetEvent::Dont(telopt)]
            }
            QState::WantNo => {
                self.set_state(telopt, QState::No, state.him);
                vec![TelnetEvent::Dont(telopt)]
            }
            QState::WantNoOpposite => {
                self.set_state(telopt, QState::WantYes, state.him);
                vec![Self::send_negotiate(TELNET_WILL, telopt), TelnetEvent::Dont(telopt)]
            }
            QState::WantYes | QState::WantYesOpposite => {
                self.set_state(telopt, QState::No, state.him);
                Vec::new()
            }
            QState::No => Vec::new(),
        }
    }

    fn negotiate_will(&mut self, telopt: u8) -> Vec<TelnetEvent> {
        self.negotiate_open(telopt, TELNET_WILL, Side::Us)
    }

    fn negotiate_wont(&mut self, telopt: u8) -> Vec<TelnetEvent> {
        self.negotiate_close(telopt, TELNET_WONT, Side::Us)
    }

    fn negotiate_do(&mut self, telopt: u8) -> Vec<TelnetEvent> {
        self.negotiate_open(telopt, TELNET_DO, Side::Him)
    }

    fn negotiate_dont(&mut self, telopt: u8) -> Vec<TelnetEvent> {
        self.negotiate_close(telopt, TELNET_DONT, Side::Him)
    }

    fn negotiate_open(&mut self, telopt: u8, command: u8, side: Side) -> Vec<TelnetEvent> {
        let state = self.state(telopt);
        match side.q(state) {
            QState::No => {
                self.set_side_state(telopt, side, state, QState::WantYes);
                vec![Self::send_negotiate(command, telopt)]
            }
            QState::WantNo => {
                self.set_side_state(telopt, side, state, QState::WantNoOpposite);
                Vec::new()
            }
            QState::WantYesOpposite => {
                self.set_side_state(telopt, side, state, QState::WantYes);
                Vec::new()
            }
            _ => Vec::new(),
        }
    }

    fn negotiate_close(&mut self, telopt: u8, command: u8, side: Side) -> Vec<TelnetEvent> {
        let state = self.state(telopt);
        match side.q(state) {
            QState::Yes => {
                self.set_side_state(telopt, side, state, QState::WantNo);
                vec![Self::send_negotiate(command, telopt)]
            }
            QState::WantYes => {
                self.set_side_state(telopt, side, state, QState::WantYesOpposite);
                Vec::new()
            }
            QState::WantNoOpposite => {
                self.set_side_state(telopt, side, state, QState::WantNo);
                Vec::new()
            }
            _ => Vec::new(),
        }
    }

    fn set_side_state(&mut self, telopt: u8, side: Side, state: TelnetOptionState, q: QState) {
        match side {
            Side::Us => self.set_state(telopt, q, state.him),
            Side::Him => self.set_state(telopt, state.us, q),
        }
    }

    fn set_state(&mut self, telopt: u8, us: QState, him: QState) {
        self.states.insert(telopt, TelnetOptionState { us, him });
        if telopt == TELNET_TELOPT_BINARY {
            self.flags &= !(TELNET_FLAG_TRANSMIT_BINARY | TELNET_FLAG_RECEIVE_BINARY);
            if us == QState::Yes {
                self.flags |= TELNET_FLAG_TRANSMIT_BINARY;
            }
            if him == QState::Yes {
                self.flags |= TELNET_FLAG_RECEIVE_BINARY;
            }
        }
    }

    fn send_negotiate(command: u8, telopt: u8) -> TelnetEvent {
        TelnetEvent::Send(vec![TELNET_IAC, command, telopt])
    }

    fn protocol_warning(message: &str) -> TelnetEvent {
        TelnetEvent::Warning(TelnetProtocolError::new(TelnetError::Protocol, message.to_string()))
    }
}
