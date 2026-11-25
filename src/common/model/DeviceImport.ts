export const DEVICE_IMPORT_SOURCE = {
    SSH_CONFIG: 1,
    PUTTY: 2,
    XSHELL: 3,
    TERMIUS: 4,
} as const;

export const DEVICE_IMPORT_PASTE_SOURCES: number[] = [DEVICE_IMPORT_SOURCE.SSH_CONFIG, DEVICE_IMPORT_SOURCE.TERMIUS];

export interface DeviceImportItem {
    name: string;
    host: string;
    port: number;
    username?: string | null;
    jump_name?: string | null;
    exists: boolean;
}

export interface DeviceImportApplyResult {
    imported: number;
    skipped: number;
    jump_linked: number;
}
