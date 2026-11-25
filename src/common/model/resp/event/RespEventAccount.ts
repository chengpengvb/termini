export const RESP_EVENT_ACCOUNT = {
    ON_LOGIN: 140,
    ON_LOGOUT: 141,
    ON_LOGIN_ERROR: 143,
} as const;

const ACCOUNT_EVENT_TYPES: ReadonlySet<number> = new Set(Object.values(RESP_EVENT_ACCOUNT));

export function isAccountEvent(type: number): boolean {
    return ACCOUNT_EVENT_TYPES.has(type);
}
