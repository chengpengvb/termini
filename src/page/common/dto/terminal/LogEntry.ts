export const LOG_LEVEL = {
    INFO: 1,
    ERROR: 2,
} as const;

export interface LogEntry {
    id: number;
    level: number;
    text: string;
    timestamp: number;
}
