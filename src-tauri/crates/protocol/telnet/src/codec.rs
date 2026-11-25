use crate::command::{
    TELNET_DO, TELNET_DONT, TELNET_EOR, TELNET_FLAG_NVT_EOL, TELNET_FLAG_PROXY, TELNET_FLAG_RECEIVE_BINARY, TELNET_FLAG_TRANSMIT_BINARY, TELNET_IAC, TELNET_SB, TELNET_SE, TELNET_TELOPT_COMPRESS,
    TELNET_TELOPT_COMPRESS2, TELNET_TELOPT_ENVIRON, TELNET_TELOPT_MSSP, TELNET_TELOPT_NEW_ENVIRON, TELNET_TELOPT_TTYPE, TELNET_TELOPT_ZMP, TELNET_WILL, TELNET_WONT,
};
use crate::zmp::parse_zmp;
use crate::mssp::parse_mssp;
use crate::event::TelnetEvent;
use crate::environ::parse_environ;
use crate::negotiation::TelnetNegotiator;
use crate::subnegotiation::parse_terminal_type;
use crate::error::{TelnetError, TelnetProtocolError};
use crate::compress::{TelnetCompressionStatus, TelnetCompressor};
use crate::option::{TelnetOptionSupport, DEFAULT_TELNET_OPTIONS};

const DEFAULT_SUBNEGOTIATION_BUFFER_LIMIT: usize = 16_384;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum TelnetState {
    Data,
    Eol,
    Iac,
    Will,
    Wont,
    Do,
    Dont,
    Subnegotiation,
    SubnegotiationData,
    SubnegotiationDataIac,
}

#[derive(Debug)]
pub struct TelnetCodec {
    state: TelnetState,
    subnegotiation_telopt: u8,
    subnegotiation_buffer: Vec<u8>,
    subnegotiation_buffer_limit: usize,
    skip_mccpv1_marker_tail: bool,
    negotiator: TelnetNegotiator,
    compressor: TelnetCompressor,
}

impl Default for TelnetCodec {
    fn default() -> Self {
        Self::new(DEFAULT_TELNET_OPTIONS.to_vec(), 0)
    }
}

impl TelnetCodec {
    pub fn new(telopts: Vec<TelnetOptionSupport>, flags: u8) -> Self {
        Self {
            state: TelnetState::Data,
            subnegotiation_telopt: 0,
            subnegotiation_buffer: Vec::new(),
            subnegotiation_buffer_limit: DEFAULT_SUBNEGOTIATION_BUFFER_LIMIT,
            skip_mccpv1_marker_tail: false,
            negotiator: TelnetNegotiator::new(telopts, flags),
            compressor: TelnetCompressor::new(),
        }
    }

    pub fn state(&self) -> TelnetState {
        self.state
    }

    pub fn flags(&self) -> u8 {
        self.negotiator.flags()
    }

    pub fn set_flags(&mut self, flags: u8) {
        self.negotiator.set_flags(flags);
    }

    pub fn set_subnegotiation_buffer_limit(&mut self, limit: usize) {
        self.subnegotiation_buffer_limit = limit;
    }

    pub fn receive(&mut self, input: &[u8]) -> Vec<TelnetEvent> {
        let mut events = Vec::new();
        let mut data = Vec::new();
        let mut index = 0;
        while index < input.len() {
            if self.skip_mccpv1_marker_tail {
                self.skip_mccpv1_marker_tail = false;
                index += 1;
                continue;
            }
            if self.compressor.is_inflating() {
                self.process_compressed_input(&input[index..], &mut data, &mut events);
                break;
            }
            self.process_byte(input[index], &mut data, &mut events);
            index += 1;
        }
        Self::flush_data(&mut data, &mut events);
        events
    }

    pub fn send_data(&mut self, input: &[u8]) -> TelnetEvent {
        let mut output = Vec::with_capacity(input.len());
        for byte in input {
            output.push(*byte);
            if *byte == TELNET_IAC {
                output.push(TELNET_IAC);
            }
        }
        self.send_output(output)
    }

    pub fn send_text(&mut self, input: &[u8]) -> TelnetEvent {
        let mut output = Vec::with_capacity(input.len());
        for byte in input {
            self.push_text_byte(*byte, &mut output);
        }
        self.send_output(output)
    }

    pub fn send_formatted_text(&mut self, arguments: std::fmt::Arguments<'_>) -> TelnetEvent {
        let text = format!("{}", arguments);
        self.send_text(text.as_bytes())
    }

    pub fn send_raw_formatted(&mut self, arguments: std::fmt::Arguments<'_>) -> TelnetEvent {
        let text = format!("{}", arguments);
        self.send_data(text.as_bytes())
    }

    pub fn iac(&mut self, command: u8) -> TelnetEvent {
        self.send_output(vec![TELNET_IAC, command])
    }

    pub fn begin_sb(&mut self, telopt: u8) -> TelnetEvent {
        self.send_output(vec![TELNET_IAC, TELNET_SB, telopt])
    }

    pub fn finish_sb(&mut self) -> TelnetEvent {
        self.iac(TELNET_SE)
    }

    pub fn subnegotiation(&mut self, telopt: u8, input: &[u8]) -> Vec<TelnetEvent> {
        let mut output = Vec::with_capacity(input.len() + 5);
        output.extend_from_slice(&[TELNET_IAC, TELNET_SB, telopt]);
        Self::push_escaped_data(input, &mut output);
        output.extend_from_slice(&[TELNET_IAC, TELNET_SE]);
        let mut events = vec![self.send_output(output)];
        if telopt == TELNET_TELOPT_COMPRESS2 && self.negotiator.flags() & TELNET_FLAG_PROXY != 0 {
            match self.compressor.start_deflate() {
                Ok(()) => events.push(TelnetEvent::Compress { enabled: true }),
                Err(error) => events.push(TelnetEvent::Error(error)),
            }
        }
        events
    }

    pub fn negotiate(&mut self, command: u8, telopt: u8) -> Vec<TelnetEvent> {
        self.negotiator.negotiate(command, telopt)
    }

    pub fn start_deflate(&mut self) -> Result<(), TelnetProtocolError> {
        self.compressor.start_deflate()
    }

    pub fn begin_compress2(&mut self) -> Result<Vec<TelnetEvent>, TelnetProtocolError> {
        let marker = vec![TELNET_IAC, TELNET_SB, TELNET_TELOPT_COMPRESS2, TELNET_IAC, TELNET_SE];
        self.compressor.start_deflate()?;
        Ok(vec![TelnetEvent::Send(marker), TelnetEvent::Compress { enabled: true }])
    }

    fn process_compressed_input(&mut self, input: &[u8], data: &mut Vec<u8>, events: &mut Vec<TelnetEvent>) {
        match self.compressor.inflate(input) {
            Ok((inflated, status)) => {
                self.process_plain_input(&inflated, data, events);
                if status == TelnetCompressionStatus::Disabled {
                    events.push(TelnetEvent::Compress { enabled: false });
                }
            }
            Err(error) => events.push(TelnetEvent::Error(error)),
        }
    }

    fn process_plain_input(&mut self, input: &[u8], data: &mut Vec<u8>, events: &mut Vec<TelnetEvent>) {
        for byte in input {
            self.process_byte(*byte, data, events);
        }
    }

    fn process_byte(&mut self, byte: u8, data: &mut Vec<u8>, events: &mut Vec<TelnetEvent>) {
        match self.state {
            TelnetState::Data => self.process_data(byte, data, events),
            TelnetState::Eol => self.process_eol(byte, data),
            TelnetState::Iac => self.process_iac(byte, data, events),
            TelnetState::Will => self.process_negotiation(TELNET_WILL, byte, events),
            TelnetState::Wont => self.process_negotiation(TELNET_WONT, byte, events),
            TelnetState::Do => self.process_negotiation(TELNET_DO, byte, events),
            TelnetState::Dont => self.process_negotiation(TELNET_DONT, byte, events),
            TelnetState::Subnegotiation => self.process_subnegotiation(byte),
            TelnetState::SubnegotiationData => self.process_subnegotiation_data(byte, events),
            TelnetState::SubnegotiationDataIac => self.process_subnegotiation_data_iac(byte, data, events),
        }
    }

    fn process_data(&mut self, byte: u8, data: &mut Vec<u8>, events: &mut Vec<TelnetEvent>) {
        if byte == TELNET_IAC {
            Self::flush_data(data, events);
            self.state = TelnetState::Iac;
            return;
        }
        if self.should_translate_eol(byte) {
            Self::flush_data(data, events);
            self.state = TelnetState::Eol;
            return;
        }
        data.push(byte);
    }

    fn process_eol(&mut self, byte: u8, data: &mut Vec<u8>) {
        if byte == b'\n' {
            data.push(b'\n');
        } else if byte == 0 {
            data.push(b'\r');
        } else {
            data.push(b'\r');
            data.push(byte);
        }
        self.state = TelnetState::Data;
    }

    fn process_iac(&mut self, byte: u8, data: &mut Vec<u8>, events: &mut Vec<TelnetEvent>) {
        match byte {
            TELNET_SB => self.state = TelnetState::Subnegotiation,
            TELNET_WILL => self.state = TelnetState::Will,
            TELNET_WONT => self.state = TelnetState::Wont,
            TELNET_DO => self.state = TelnetState::Do,
            TELNET_DONT => self.state = TelnetState::Dont,
            TELNET_IAC => {
                data.push(TELNET_IAC);
                self.state = TelnetState::Data;
            }
            TELNET_EOR => {
                events.push(TelnetEvent::Eor);
                self.state = TelnetState::Data;
            }
            _ => {
                events.push(TelnetEvent::Iac(byte));
                self.state = TelnetState::Data;
            }
        }
    }

    fn process_negotiation(&mut self, command: u8, telopt: u8, events: &mut Vec<TelnetEvent>) {
        events.extend(self.negotiator.receive(command, telopt));
        self.state = TelnetState::Data;
    }

    fn process_subnegotiation(&mut self, telopt: u8) {
        self.subnegotiation_telopt = telopt;
        self.subnegotiation_buffer.clear();
        self.state = TelnetState::SubnegotiationData;
    }

    fn process_subnegotiation_data(&mut self, byte: u8, events: &mut Vec<TelnetEvent>) {
        if byte == TELNET_IAC {
            self.state = TelnetState::SubnegotiationDataIac;
            return;
        }
        if byte == TELNET_WILL && self.is_mccpv1_begin_marker() {
            self.discard_mccpv1_begin_marker();
            return;
        }
        self.push_subnegotiation_byte(byte, events);
    }

    fn process_subnegotiation_data_iac(&mut self, byte: u8, data: &mut Vec<u8>, events: &mut Vec<TelnetEvent>) {
        match byte {
            TELNET_SE => self.finish_subnegotiation(events),
            TELNET_IAC => self.push_subnegotiation_escaped_iac(events),
            _ => self.handle_unexpected_subnegotiation_iac(byte, data, events),
        }
    }

    fn is_mccpv1_begin_marker(&self) -> bool {
        self.subnegotiation_telopt == TELNET_TELOPT_COMPRESS
    }

    fn discard_mccpv1_begin_marker(&mut self) {
        self.subnegotiation_buffer.clear();
        self.skip_mccpv1_marker_tail = true;
        self.state = TelnetState::Data;
    }

    fn push_subnegotiation_escaped_iac(&mut self, events: &mut Vec<TelnetEvent>) {
        self.push_subnegotiation_byte(TELNET_IAC, events);
        if self.state == TelnetState::SubnegotiationDataIac {
            self.state = TelnetState::SubnegotiationData;
        }
    }

    fn handle_unexpected_subnegotiation_iac(&mut self, byte: u8, data: &mut Vec<u8>, events: &mut Vec<TelnetEvent>) {
        events.push(Self::warning(TelnetError::Protocol, format!("unexpected byte after IAC inside SB: {}", byte)));
        self.finish_subnegotiation(events);
        self.state = TelnetState::Iac;
        self.process_iac(byte, data, events);
    }

    fn push_subnegotiation_byte(&mut self, byte: u8, events: &mut Vec<TelnetEvent>) {
        if self.subnegotiation_buffer.len() >= self.subnegotiation_buffer_limit {
            events.push(Self::warning(TelnetError::Overflow, "subnegotiation buffer size limit reached".to_string()));
            self.subnegotiation_buffer.clear();
            self.state = TelnetState::Data;
            return;
        }
        self.subnegotiation_buffer.push(byte);
        self.state = TelnetState::SubnegotiationData;
    }

    fn finish_subnegotiation(&mut self, events: &mut Vec<TelnetEvent>) {
        let telopt = self.subnegotiation_telopt;
        let data = self.subnegotiation_buffer.clone();
        events.push(TelnetEvent::Subnegotiation { telopt, data: data.clone() });
        self.push_subprotocol_event(telopt, &data, events);
        self.subnegotiation_buffer.clear();
        self.state = TelnetState::Data;
    }

    fn push_subprotocol_event(&mut self, telopt: u8, data: &[u8], events: &mut Vec<TelnetEvent>) {
        let parsed_event = match telopt {
            TELNET_TELOPT_TTYPE => parse_terminal_type(data),
            TELNET_TELOPT_ENVIRON | TELNET_TELOPT_NEW_ENVIRON => parse_environ(telopt, data),
            TELNET_TELOPT_MSSP => parse_mssp(data),
            TELNET_TELOPT_ZMP => parse_zmp(data),
            TELNET_TELOPT_COMPRESS2 => return self.start_inflate(events),
            _ => return,
        };
        match parsed_event {
            Ok(event) => events.push(event),
            Err(error) => events.push(TelnetEvent::Warning(error)),
        }
    }

    fn start_inflate(&mut self, events: &mut Vec<TelnetEvent>) {
        match self.compressor.start_inflate() {
            Ok(()) => events.push(TelnetEvent::Compress { enabled: true }),
            Err(error) => events.push(TelnetEvent::Error(error)),
        }
    }

    fn send_output(&mut self, output: Vec<u8>) -> TelnetEvent {
        match self.compressor.deflate(&output) {
            Ok(output) => TelnetEvent::Send(output),
            Err(error) => TelnetEvent::Error(error),
        }
    }

    fn push_text_byte(&self, byte: u8, output: &mut Vec<u8>) {
        if byte == TELNET_IAC {
            output.push(TELNET_IAC);
            output.push(TELNET_IAC);
        } else if self.should_translate_send_text(byte) {
            Self::push_nvt_eol(byte, output);
        } else {
            output.push(byte);
        }
    }

    fn should_translate_eol(&self, byte: u8) -> bool {
        byte == b'\r' && self.negotiator.flags() & TELNET_FLAG_NVT_EOL != 0 && self.negotiator.flags() & TELNET_FLAG_RECEIVE_BINARY == 0
    }

    fn should_translate_send_text(&self, byte: u8) -> bool {
        self.negotiator.flags() & TELNET_FLAG_NVT_EOL != 0 && self.negotiator.flags() & TELNET_FLAG_TRANSMIT_BINARY == 0 && (byte == b'\r' || byte == b'\n')
    }

    fn push_nvt_eol(byte: u8, output: &mut Vec<u8>) {
        if byte == b'\r' {
            output.extend_from_slice(&[b'\r', 0]);
        } else {
            output.extend_from_slice(&[b'\r', b'\n']);
        }
    }

    fn push_escaped_data(input: &[u8], output: &mut Vec<u8>) {
        for byte in input {
            output.push(*byte);
            if *byte == TELNET_IAC {
                output.push(TELNET_IAC);
            }
        }
    }

    fn flush_data(data: &mut Vec<u8>, events: &mut Vec<TelnetEvent>) {
        if data.is_empty() {
            return;
        }
        events.push(TelnetEvent::Data(data.clone()));
        data.clear();
    }

    fn warning(kind: TelnetError, message: String) -> TelnetEvent {
        TelnetEvent::Warning(TelnetProtocolError::new(kind, message))
    }
}
