export const CLOUD_SYNC_OP = {
    UPLOAD: 1,
    DOWNLOAD: 2,
} as const;

export const CLOUD_SYNC_SIDE = {
    LOCAL: 1,
    CLOUD: 2,
} as const;

export const CLOUD_SYNC_TYPE = {
    DEVICE: 1,
    GROUP: 2,
    GROUP_2_DEVICE: 3,
    CONFIG_SSH: 4,
    CONFIG_TELNET: 5,
    KNOWN_HOST: 6,
    CONFIG_KEY_PAIR: 7,
    PORT_FORWARD: 8,
    JOB_DEVICE: 9,
    JOB_INDICATOR: 10,
    JOB_DEVICE_2_INDICATOR: 11,
    AI_SKILL: 12,
    AI_MCP_SERVER: 13,
    CONFIG_RDP: 14,
    CONFIG_SNMP: 15,
    SNIPPET: 16,
    CONFIG_SERIAL: 17,
    CONFIG_FTP: 37,
} as const;

export const CLOUD_SYNC_PASSWORD_MIN_LEN = 6;
export const CLOUD_SYNC_PASSWORD_MAX_LEN = 20;

export interface CloudSyncStatus {
    hasBackup: boolean;
    updateTime: number;
}

export interface CloudSyncConflict {
    sync_type: number;
    id: string;
    local: Record<string, unknown>;
    cloud: Record<string, unknown>;
}

export interface CloudSyncChoice {
    sync_type: number;
    id: string;
    side: number;
}
