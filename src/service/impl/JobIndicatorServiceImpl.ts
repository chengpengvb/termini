import {inject, injectable} from 'inversify';
import {API_BASE_URL} from '@/runtime-config.ts';
import {BaseApiServiceImpl} from './BaseApiServiceImpl';
import {ENTITY_TYPE} from '@/common/const/EntityTypeConst.ts';
import type {JobIndicatorService} from '../JobIndicatorService';
import type {JobDevice2Indicator} from '@/common/model/JobDevice.ts';
import type {JobIndicatorValue} from '@/common/model/JobIndicatorValue.ts';
import type {ReqJobIndicator} from '@/common/model/req/ReqJobIndicator.ts';
import {WebSocketServiceImpl} from '@/service/impl/WebSocketServiceImpl.ts';
import type {IndicatorArgTemplate, JobIndicator} from '@/common/model/JobIndicator.ts';

@injectable()
export class JobIndicatorServiceImpl extends BaseApiServiceImpl<JobIndicator> implements JobIndicatorService {
    constructor(
        @inject(WebSocketServiceImpl) webSocketService: WebSocketServiceImpl,
    ) {
        super(webSocketService);
    }

    protected getNotifyEntityType(): number {
        return ENTITY_TYPE.JOB_INDICATOR;
    }

    protected getPrefix(): string {
        return '/job_indicator';
    }

    async selectArgTemplate(): Promise<IndicatorArgTemplate[]> {
        const data = await this.request<IndicatorArgTemplate[]>(this.buildUrl('/select_arg_template'));
        if (!data) {
            return [];
        }
        return data;
    }

    async selectDefault(): Promise<JobIndicator[]> {
        const data = await this.request<JobIndicator[]>(this.buildUrl('/select_default'));
        if (!data) {
            return [];
        }
        return data;
    }

    async saveWithJob(req: ReqJobIndicator): Promise<JobDevice2Indicator> {
        const data = await this.request<JobDevice2Indicator>(this.buildUrl('/save_with_job'), {
            method: 'POST',
            headers: this.JSON_HEADERS,
            body: JSON.stringify(req),
        });
        if (!data) {
            throw new Error('saveWithJob returned no data');
        }
        return data;
    }

    async deleteWithJob(jobId: string): Promise<void> {
        await this.request<unknown>(this.buildUrl(`/delete_with_job/${jobId}`), {method: 'DELETE'});
    }

    async runOnce(jobId: string): Promise<JobIndicatorValue[]> {
        const data = await this.request<JobIndicatorValue[]>(`${API_BASE_URL}/job_device_2_indicator/run_once/${jobId}`, {
            method: 'POST',
        });
        if (!data) {
            return [];
        }
        return data;
    }
}
