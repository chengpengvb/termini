export const RESP_EVENT_JOB_ALERT = {
    ON_CHANGED: 96,
} as const;

const JOB_ALERT_EVENT_TYPES: ReadonlySet<number> = new Set(Object.values(RESP_EVENT_JOB_ALERT));

export function isJobAlertEvent(type: number): boolean {
    return JOB_ALERT_EVENT_TYPES.has(type);
}
