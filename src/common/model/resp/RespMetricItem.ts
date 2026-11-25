import type {JobIndicatorAlert} from '../JobIndicatorAlert';
import type {JobIndicatorValue} from '../JobIndicatorValue';

export const RESP_METRIC_STATUS = {
    PENDING: 0,
    OK: 1,
    ERROR: 2,
} as const;

export interface RespMetricItem {
    status: number;
    job_device_id: string;
    updated_at?: number | null;
    job_indicator_id: string;
    values: JobIndicatorValue[];
    alert?: JobIndicatorAlert | null;
}
