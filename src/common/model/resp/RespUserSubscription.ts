export interface UserSubscription {
    plan: number;
    store: number;
    startTime: number;
    expireTime: number;
    monthlyQuota: number;
    usedQuota: number;
    balanceQuota: number;
    quotaResetTime: number;
}
