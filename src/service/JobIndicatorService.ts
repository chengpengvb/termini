import type { ApiService } from './ApiService';
import type { JobDevice2Indicator } from '@/common/model/JobDevice.ts';
import type { JobIndicatorValue } from '@/common/model/JobIndicatorValue.ts';
import type { ReqJobIndicator } from '@/common/model/req/ReqJobIndicator.ts';
import type { IndicatorArgTemplate, JobIndicator } from '@/common/model/JobIndicator.ts';

export interface JobIndicatorService extends ApiService<JobIndicator> {

    selectDefault(): Promise<JobIndicator[]>;

    deleteWithJob(jobId: string): Promise<void>;

    runOnce(jobId: string): Promise<JobIndicatorValue[]>;

    selectArgTemplate(): Promise<IndicatorArgTemplate[]>;

    saveWithJob(req: ReqJobIndicator): Promise<JobDevice2Indicator>;

}
