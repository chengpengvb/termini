import {inject, injectable} from 'inversify';
import {BaseApiServiceImpl} from './BaseApiServiceImpl';
import {ENTITY_TYPE} from '@/common/const/EntityTypeConst.ts';
import type {JobIndicatorValueService} from '../JobIndicatorValueService';
import type {JobIndicatorValue} from '@/common/model/JobIndicatorValue.ts';
import {WebSocketServiceImpl} from '@/service/impl/WebSocketServiceImpl.ts';

@injectable()
export class JobIndicatorValueServiceImpl extends BaseApiServiceImpl<JobIndicatorValue> implements JobIndicatorValueService {
    constructor(
        @inject(WebSocketServiceImpl) webSocketService: WebSocketServiceImpl,
    ) {
        super(webSocketService);
    }

    protected getNotifyEntityType(): number {
        return ENTITY_TYPE.NONE;
    }

    protected getPrefix(): string {
        return '/job_indicator_value';
    }

    async selectByRange(jobDeviceId: string, jobIndicatorId: string, durationMs: number): Promise<JobIndicatorValue[]> {
        const data = await this.request<JobIndicatorValue[]>(this.buildUrl(`/select_by_job_device_id/${jobDeviceId}/job_indicator_id/${jobIndicatorId}/range/${durationMs}`));
        if (!data) {
            return [];
        }
        return data;
    }
}
