export const AI_CHAT_TYPE = {
    CHAT: 1,
    DISCOVER: 2,
    ALERT: 3,
} as const;

export const AI_CHAT_STATUS = {
    NONE: 0,
    QUEUED: 1,
    RUNNING: 2,
    SUCCESS: 3,
    FAILED: 4,
    CANCELLED: 5,
} as const;

export interface AiChat {
    id: string;
    sort: number;
    title: string;
    timestamp: number;
    type: number;
    status: number;
    error?: string | null;
    ended_at?: number | null;
    summary?: string | null;
    biz_id?: string | null;
    ai_provide_id?: string | null;
}
