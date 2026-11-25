export const TERMINAL_KIND = {
    SHELL: 'shell',
    SSH: 'ssh',
    TELNET: 'telnet',
    SERIAL: 'serial',
} as const;

export interface ReqCommandSuggest {
    deviceId: string;
    altScreen: boolean;
    terminalKind: string;
    shellKind?: string;
    currentLine: string;
    recentOutput: string;
    history: string[];
    instruction?: string;
    suggestCount: number;
}
