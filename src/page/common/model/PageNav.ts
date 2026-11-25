export const PAGE_NAV_DIRECTION = {
    FORWARD: 1,
    BACK: 2,
} as const;

export type PageNavDirection = typeof PAGE_NAV_DIRECTION[keyof typeof PAGE_NAV_DIRECTION];
