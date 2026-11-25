export const RESP_EVENT_WINDOW = {
    ON_OPEN_HERE: 96,
    ON_CLOSE_ASK: 94,
    ON_TRAY_ACTION: 95,
    ON_MAXIMIZED_CHANGED: 93,
} as const;

export type OpenHerePayload = {
    path: string;
    seq: number;
};

export const TRAY_ACTION = {
    HOME: 0,
    NEW_CHAT: 1,
} as const;

const WINDOW_EVENT_TYPES: ReadonlySet<number> = new Set(Object.values(RESP_EVENT_WINDOW));

export function isWindowEvent(type: number): boolean {
    return WINDOW_EVENT_TYPES.has(type);
}
