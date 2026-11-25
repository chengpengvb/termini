import type {Device} from './Device.ts';
import type {JobIndicator} from './JobIndicator.ts';

export interface JobDevice2Indicator {
    id: string;
    interval_secs: number;
    job_device_id: string;
    job_indicator_id: string;
    job_indicator?: JobIndicator | null;
}

export interface JobDevice {
    id: string;
    device_id: string;
    enabled: boolean;
    updated_at: number;
    device?: Device | null;
    indicators?: JobDevice2Indicator[] | null;
}
