import type {JobIndicator} from '../JobIndicator.ts';

export interface ReqJobIndicator {
    interval_secs: number;
    job_device_id: string;
    job_indicator: JobIndicator;
}
