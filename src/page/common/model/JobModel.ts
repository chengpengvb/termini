export const JOB_TAB = {
    OVERVIEW: 0,
    MONITOR: 1,
    DISCOVER: 2,
} as const;

export type JobTab = typeof JOB_TAB[keyof typeof JOB_TAB];

export const JOB_MONITOR_VIEW = {
    LIST: 1,
    GRID: 2,
} as const;

export type JobMonitorView = typeof JOB_MONITOR_VIEW[keyof typeof JOB_MONITOR_VIEW];

export const JOB_DEVICE_DRAWER_TAB = {
    DETAIL: 0,
    ALERT: 1,
} as const;

export type JobDeviceDrawerTab = typeof JOB_DEVICE_DRAWER_TAB[keyof typeof JOB_DEVICE_DRAWER_TAB];

export const JOB_DISCOVER_DRAWER_TAB = {
    RESULT: 0,
    ECHO: 1,
} as const;

export type JobDiscoverDrawerTab = typeof JOB_DISCOVER_DRAWER_TAB[keyof typeof JOB_DISCOVER_DRAWER_TAB];
