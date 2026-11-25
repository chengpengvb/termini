import type {UserSubscription} from '@/common/model/resp/RespUserSubscription.ts';

export interface UserAccount {
    id?: string;
    email?: string | null;
    creditScale?: number | null;
    subscription?: UserSubscription | null;
}
