export const VIEW_MODE = {
    GRID: 1,
    LIST: 2,
} as const;

export const PROTOCOL = {
    TELNET: 1,
    SSH: 2,
    SFTP: 3,
    RDP: 4,
    SNMP: 5,
    SERIAL: 6,
    FTP: 7,
} as const;

export const TERMINAL_CURSOR_STYLE = {
    BAR: 'bar',
    BLOCK: 'block',
} as const;

export const TERMINAL_SELECT_HANDLE = {
    START: 1,
    END: 2,
} as const;

export const TERMINAL_HANDLE_FACE = {
    LEFT: 1,
    RIGHT: 2,
} as const;

export const TERMINAL_POINTER = {
    DOWN: 1,
    MOVE: 2,
    UP: 3,
    CANCEL: 4,
    MENU: 5,
    HOLD: 6,
} as const;

export const TERMINAL_POINTER_MODE = {
    PENDING: 1,
    SCROLLING: 2,
    SELECTING: 3,
} as const;
