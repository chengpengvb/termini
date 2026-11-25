export interface SuggestContext {
    deviceId: string;
    terminalKind: string;
    shellId?: string;
    shellKind?: string;
    sessionOpened?: boolean;
}
