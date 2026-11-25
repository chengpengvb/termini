import {translate} from '@/common/i18n/i18n.ts';
import type {JobIndicator} from './JobIndicator.ts';
import type {JobIndicatorValue} from './JobIndicatorValue.ts';

export const JOB_ALERT_STATUS = {
    FIRING: 1,
    NORMAL: 2,
} as const;

export const JOB_ALERT_FIRE_TYPE = {
    COLLECT_FAILED: 1,
    RULE: 2,
} as const;

export const JOB_ALERT_RESOLVE_TYPE = {
    RULE: 1,
    ALERT_OFF: 2,
} as const;

export interface JobIndicatorAlert {
    id: string;
    status: number;
    alert_at: number;
    alert_value: string;
    alert_rule_op: number;
    job_device_id: string;
    alert_threshold: string;
    job_indicator_id: string;
    fire_type?: number | null;
    ai_summary?: string | null;
    ai_chat_id?: string | null;
    resolved_at?: number | null;
    resolved_type?: number | null;
    resolved_value?: string | null;
    job_indicator?: JobIndicator | null;
    job_indicator_value: JobIndicatorValue | null;
}

export function fireTypeLabel(fireType: number | null | undefined): string {
    if (fireType === JOB_ALERT_FIRE_TYPE.COLLECT_FAILED || fireType === JOB_ALERT_FIRE_TYPE.RULE) {
        return translate(`home.job.alert.fireType.${fireType}`);
    }
    return '—';
}

export function resolveTypeLabel(resolveType: number | null | undefined): string {
    if (resolveType === JOB_ALERT_RESOLVE_TYPE.RULE || resolveType === JOB_ALERT_RESOLVE_TYPE.ALERT_OFF) {
        return translate(`home.job.alert.resolveType.${resolveType}`);
    }
    return '—';
}
