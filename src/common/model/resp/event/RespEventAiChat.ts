export const RESP_EVENT_AI_CHAT = {
    ON_CHANGED: 90,
    ON_DELETED: 91,
} as const;

const AI_CHAT_EVENT_TYPES: ReadonlySet<number> = new Set(Object.values(RESP_EVENT_AI_CHAT));

export function isAiChatEvent(type: number): boolean {
    return AI_CHAT_EVENT_TYPES.has(type);
}
