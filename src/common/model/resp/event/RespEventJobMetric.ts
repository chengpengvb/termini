export const RESP_EVENT_JOB_METRIC = {
    ON_CHANGED: 74,
} as const;

const JOB_METRIC_EVENT_TYPES: ReadonlySet<number> = new Set(Object.values(RESP_EVENT_JOB_METRIC));

export function isJobMetricEvent(type: number): boolean {
    return JOB_METRIC_EVENT_TYPES.has(type);
}
