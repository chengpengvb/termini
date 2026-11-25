export const RESP_EVENT_AI_FILE_CHANGE = {
    ON_CHANGED: 97,
} as const;

export function isAiFileChangeEvent(type: number): boolean {
    return type === RESP_EVENT_AI_FILE_CHANGE.ON_CHANGED;
}
