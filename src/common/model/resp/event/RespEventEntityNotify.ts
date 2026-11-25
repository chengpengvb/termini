export const RESP_EVENT_ENTITY_NOTIFY = {
    ON_CHANGED: 105,
} as const;

const ENTITY_NOTIFY_EVENT_TYPES: ReadonlySet<number> = new Set(Object.values(RESP_EVENT_ENTITY_NOTIFY));

export function isEntityNotifyEvent(type: number): boolean {
    return ENTITY_NOTIFY_EVENT_TYPES.has(type);
}
