import {translate} from '@/common/i18n/i18n.ts';

export const SSH_DEFAULT_PORT = 22;
export const FTP_DEFAULT_PORT = 21;
export const RDP_DEFAULT_PORT = 3389;
export const TELNET_DEFAULT_PORT = 23;
export const SNMP_DEFAULT_PORT = 161;
export const SNMP_DEFAULT_VERSION = 2;
export const SNMP_DEFAULT_RETRIES = 1;
export const SNMP_DEFAULT_TIMEOUT_SECONDS = 3;
export const SNMP_DEFAULT_COMMUNITY = 'public';
export const SERIAL_BAUD_RATE_OPTIONS = [
    {value: '1200', label: '1200'},
    {value: '2400', label: '2400'},
    {value: '4800', label: '4800'},
    {value: '9600', label: '9600'},
    {value: '19200', label: '19200'},
    {value: '38400', label: '38400'},
    {value: '57600', label: '57600'},
    {value: '115200', label: '115200'},
    {value: '230400', label: '230400'},
    {value: '460800', label: '460800'},
    {value: '921600', label: '921600'},
];

export const SERIAL_DATA_BITS_OPTIONS = [
    {value: '5', label: '5'},
    {value: '6', label: '6'},
    {value: '7', label: '7'},
    {value: '8', label: '8'},
];

export const SERIAL_STOP_BITS_OPTIONS = [
    {value: '1', label: '1'},
    {value: '2', label: '2'},
];

export const SERIAL_PARITY_OPTIONS = [
    {value: 'none', label: 'None'},
    {value: 'odd', label: 'Odd'},
    {value: 'even', label: 'Even'},
];

export const SERIAL_FLOW_CONTROL_OPTIONS = [
    {value: 'none', label: 'None'},
    {value: 'software', label: 'XON/XOFF'},
    {value: 'hardware', label: 'RTS/CTS'},
];
export const RDP_DEFAULT_COLOR_DEPTH = 32;
export const RDP_DEFAULT_KEYBOARD_LAYOUT = 0x0409;
export const RDP_PERFORMANCE_SMOOTH = 0x6f;
export const RDP_PERFORMANCE_BALANCED = 0x06;
export const RDP_PERFORMANCE_QUALITY = 0;

const RDP_KEYBOARD_LAYOUT_KEYS: ReadonlyArray<[number, string]> = [
    [0x0409, 'enUs'],
    [0x0804, 'zhCn'],
    [0x0404, 'zhTw'],
    [0x0C04, 'zhHk'],
    [0x0411, 'jaJp'],
    [0x0412, 'koKr'],
    [0x0407, 'deDe'],
    [0x040C, 'frFr'],
    [0x0410, 'itIt'],
    [0x0C0A, 'esEs'],
    [0x0416, 'ptBr'],
    [0x0419, 'ruRu'],
];

const RDP_PERFORMANCE_KEYS: ReadonlyArray<[string, string]> = [
    ['', 'auto'],
    [String(RDP_PERFORMANCE_SMOOTH), 'smooth'],
    [String(RDP_PERFORMANCE_BALANCED), 'balanced'],
    [String(RDP_PERFORMANCE_QUALITY), 'quality'],
];

export function rdpKeyboardLayoutOptions(): {value: string; label: string}[] {
    return RDP_KEYBOARD_LAYOUT_KEYS.map(([code, key]) => ({value: String(code), label: translate(`home.devices.rdp.keyboardLayouts.${key}`)}));
}

export function rdpPerformanceOptions(): {value: string; label: string}[] {
    return RDP_PERFORMANCE_KEYS.map(([value, key]) => ({value, label: translate(`home.devices.rdp.performance.${key}`)}));
}
export const TELNET_DEFAULT_TERM = 'vt100';
export const TELNET_DEFAULT_MAX_SESSIONS = 5;
export const TELNET_DEFAULT_MORE_RESPONSE = ' ';
export const TELNET_DEFAULT_CMD_VERIFY_TIMEOUT = 10;
export const SSH_DEFAULT_TERM = 'xterm-256color';
export const TELNET_DEFAULT_PROMPT_PRI_PATTERN = '(?m)#\\s*$';
export const TELNET_DEFAULT_PROMPT_ALT_PATTERN = '(?m)[>\\]]\\s*$';
export const TELNET_DEFAULT_PASSWORD_PATTERN = '(?i)assword';
export const TELNET_DEFAULT_USERNAME_PATTERN = '(?i)(?:user:|username|login|user name)';
export const TELNET_DEFAULT_MORE_PATTERN = '(?i)-{2,}\\s*\\(?\\s*more[^\\r\\n-]*\\)?\\s*-{2,}';

export const TERMINAL_OPTIONS = [
    { value: 'xterm-256color', label: 'xterm-256color' },
    { value: 'xterm', label: 'xterm' },
    { value: 'xterm-color', label: 'xterm-color' },
    { value: 'xterm-new', label: 'xterm-new' },
    { value: 'xterm-16color', label: 'xterm-16color' },
    { value: 'xterm-88color', label: 'xterm-88color' },
    { value: 'xterm-direct', label: 'xterm-direct' },
    { value: 'screen', label: 'screen' },
    { value: 'screen-256color', label: 'screen-256color' },
    { value: 'tmux', label: 'tmux' },
    { value: 'tmux-256color', label: 'tmux-256color' },
    { value: 'vt100', label: 'vt100' },
    { value: 'vt102', label: 'vt102' },
    { value: 'vt200', label: 'vt200' },
    { value: 'vt220', label: 'vt220' },
    { value: 'ansi', label: 'ansi' },
    { value: 'linux', label: 'linux' },
    { value: 'rxvt', label: 'rxvt' },
    { value: 'rxvt-256color', label: 'rxvt-256color' },
    { value: 'putty', label: 'putty' },
    { value: 'putty-256color', label: 'putty-256color' },
    { value: 'dumb', label: 'dumb' },
];
