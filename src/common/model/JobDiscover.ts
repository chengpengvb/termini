import {translate} from '@/common/i18n/i18n.ts';
import type {IndicatorArg} from './JobIndicator.ts';

export const JOB_DISCOVER_STATUS = {
    QUEUED: 1,
    RUNNING: 2,
    SUCCESS: 3,
    FAILED: 4,
    CANCELLED: 5,
} as const;

export const JOB_DISCOVER_ITEM_STATUS = {
    CANDIDATE: 1,
    ACCEPTED: 2,
} as const;

export interface JobDiscover {
    id: string;
    chat_id: string;
    job_device_id: string;
    status: number;
    timestamp: number;
    ended_at?: number | null;
    summary?: string | null;
    error?: string | null;
    ai_provide_id?: string | null;
}

export interface JobDiscoverItem {
    id: string;
    name: string;
    status: number;
    protocol: number;
    discover_id: string;
    timestamp: number;
    parser_type: number;
    parse_expr: string;
    display_unit: number;
    job_device_id: string;
    interval_secs: number;
    alert_rule_op: number;
    reason?: string | null;
    transform_expr: string;
    alert_threshold: string;
    range_min: number | null;
    range_max: number | null;
    arg: IndicatorArg | null;
    sample_value?: string | null;
    job_indicator_id?: string | null;
}

export function isDiscoverRunning(status: number): boolean {
    return status === JOB_DISCOVER_STATUS.QUEUED || status === JOB_DISCOVER_STATUS.RUNNING;
}

export function discoverStatusLabel(status: number): string {
    const known = status === JOB_DISCOVER_STATUS.QUEUED || status === JOB_DISCOVER_STATUS.RUNNING || status === JOB_DISCOVER_STATUS.SUCCESS || status === JOB_DISCOVER_STATUS.FAILED;
    return translate(`home.job.discover.discoverStatus.${known ? status : JOB_DISCOVER_STATUS.CANCELLED}`);
}
