export const RESP_EVENT_JOB_DISCOVER = {
    ON_CHANGED: 92,
} as const;

const JOB_DISCOVER_EVENT_TYPES: ReadonlySet<number> = new Set(Object.values(RESP_EVENT_JOB_DISCOVER));

export function isJobDiscoverEvent(type: number): boolean {
    return JOB_DISCOVER_EVENT_TYPES.has(type);
}
