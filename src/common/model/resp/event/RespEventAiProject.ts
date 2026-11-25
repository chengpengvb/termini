export const RESP_EVENT_AI_PROJECT = {
    ON_CHANGED: 92,
    ON_DELETED: 93,
} as const;

const AI_PROJECT_EVENT_TYPES: ReadonlySet<number> = new Set(Object.values(RESP_EVENT_AI_PROJECT));

export function isAiProjectEvent(type: number): boolean {
    return AI_PROJECT_EVENT_TYPES.has(type);
}
