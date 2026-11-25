export const CHAT_ORIGIN = {
    USER: 0,
    ANSWER: 1,
    WAKE: 2,
    PLAN: 3,
    ALERT: 4,
    DISCOVER: 5,
    CONTINUE: 6,
} as const;

const CHAT_ORIGIN_ECHO: Record<number, boolean> = {
    [CHAT_ORIGIN.USER]: true,
    [CHAT_ORIGIN.ANSWER]: false,
    [CHAT_ORIGIN.WAKE]: false,
    [CHAT_ORIGIN.PLAN]: true,
    [CHAT_ORIGIN.ALERT]: true,
    [CHAT_ORIGIN.DISCOVER]: true,
    [CHAT_ORIGIN.CONTINUE]: false,
};

export function isChatOriginEcho(origin: number): boolean {
    return CHAT_ORIGIN_ECHO[origin];
}
