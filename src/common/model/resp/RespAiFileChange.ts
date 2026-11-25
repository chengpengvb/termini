export const FILE_CHANGE_STATE = {
    PENDING: 0,
    ACCEPTED: 1,
    REVERTED: 2,
} as const;

export interface RespAiFileChangeView {
    path: string;
    state: number;
    chatId: string;
    editCount: number;
    timestamp: number;
    deviceId: string | null;
    baselineText: string | null;
    currentText: string | null;
}
