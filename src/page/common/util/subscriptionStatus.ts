import type {UserSubscription} from '@/common/model/resp/RespUserSubscription.ts';

export const SUBSCRIPTION_PLAN = {
    FREE: 0,
    PRO: 1,
} as const;

export const SUBSCRIPTION_STORE = {
    NONE: 0,
    APPLE: 1,
    MICROSOFT: 2,
    GOOGLE: 3,
} as const;

export const SUBSCRIPTION_STATUS = {
    FREE: 'free',
    ACTIVE: 'active',
} as const;

export type SubscriptionStatus = typeof SUBSCRIPTION_STATUS[keyof typeof SUBSCRIPTION_STATUS];

export function isProActive(subscription: UserSubscription | null | undefined): boolean {
    if (subscription == null) {
        return false;
    }
    return subscription.plan === SUBSCRIPTION_PLAN.PRO && Date.now() < subscription.expireTime;
}

export function resolveSubscriptionStatus(subscription: UserSubscription | null | undefined): SubscriptionStatus | null {
    if (subscription == null) {
        return null;
    }
    if (isProActive(subscription)) {
        return SUBSCRIPTION_STATUS.ACTIVE;
    }
    return SUBSCRIPTION_STATUS.FREE;
}

export function formatCredits(quota: number, creditScale: number | null | undefined): string {
    if (creditScale == null || creditScale <= 0) {
        throw new Error('creditScale is required');
    }
    return String(Math.round(quota * creditScale));
}
