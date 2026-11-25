export const INDEX_DIR_STATE = {
    IDLE: 0,
    PENDING: 1,
    BUILDING: 2,
    READY: 3,
    FAILED: 4,
} as const;

export interface IndexDirStatus {
    dir: string;
    state: number;
    builtAt: number;
    chunkCount: number;
    stage: string | null;
    error: string | null;
}
