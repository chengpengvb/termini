import type {BaseService} from './BaseService';
import type {JobIndicatorValue} from '@/common/model/JobIndicatorValue.ts';

export interface JobIndicatorValueService extends BaseService {

    selectById(id: string): Promise<JobIndicatorValue | null>;

    selectByRange(jobDeviceId: string, jobIndicatorId: string, durationMs: number): Promise<JobIndicatorValue[]>;

}
