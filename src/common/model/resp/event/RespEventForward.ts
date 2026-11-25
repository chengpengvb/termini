export const RESP_EVENT_FORWARD = {
    ON_STARTED: 70,
    ON_STOPPED: 71,
    ON_ERROR: 72,
    ON_OPEN_STEP: 73,
} as const;

const FORWARD_EVENT_TYPES: ReadonlySet<number> = new Set(Object.values(RESP_EVENT_FORWARD));

export function isForwardEvent(type: number): boolean {
    return FORWARD_EVENT_TYPES.has(type);
}
