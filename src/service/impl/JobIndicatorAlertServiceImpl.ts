import {inject, injectable} from 'inversify';
import {BaseApiServiceImpl} from './BaseApiServiceImpl';
import {ENTITY_TYPE} from '@/common/const/EntityTypeConst.ts';
import type {JobIndicatorAlertService} from '../JobIndicatorAlertService';
import type {JobIndicatorAlert} from '@/common/model/JobIndicatorAlert.ts';
import {WebSocketServiceImpl} from '@/service/impl/WebSocketServiceImpl.ts';
import type {RespSocketContext} from '@/common/model/resp/RespWsContext.ts';
import {isJobAlertEvent} from '@/common/model/resp/event/RespEventJobAlert.ts';

@injectable()
export class JobIndicatorAlertServiceImpl extends BaseApiServiceImpl<JobIndicatorAlert> implements JobIndicatorAlertService {
    private readonly LISTENER_KEY = 'job-indicator-alert-list-change';

    constructor(
        @inject(WebSocketServiceImpl) private readonly webSocketService: WebSocketServiceImpl,
    ) {
        super(webSocketService);
        this.webSocketService.addWsListener((context: RespSocketContext) => {
            if (!isJobAlertEvent(context.type)) {
                return;
            }
            this.notifyListeners<string>(this.LISTENER_KEY, context.payload as string);
        });
    }

    protected getNotifyEntityType(): number {
        return ENTITY_TYPE.JOB_INDICATOR_ALERT;
    }

    protected getPrefix(): string {
        return '/job_indicator_alert';
    }

    addAlertListChangeListener(listener: (alertId: string) => void): () => void {
        return this.addListener<string>(this.LISTENER_KEY, listener);
    }

    async selectList(jobDeviceId: string, jobIndicatorId: string): Promise<JobIndicatorAlert[]> {
        const data = await this.request<JobIndicatorAlert[]>(
            this.buildUrl(`/select_by_job_device_id/${jobDeviceId}/job_indicator_id/${jobIndicatorId}`),
        );
        if (!data) {
            return [];
        }
        return data;
    }

    async selectRecent(limit: number): Promise<JobIndicatorAlert[]> {
        const data = await this.request<JobIndicatorAlert[]>(this.buildUrl(`/select_recent/${limit}`));
        if (!data) {
            return [];
        }
        return data;
    }

    async deleteBatch(ids: string[]): Promise<number> {
        if (ids.length === 0) {
            return 0;
        }
        const data = await this.request<number>(this.buildUrl('/delete_batch'), {
            method: 'POST',
            headers: this.JSON_HEADERS,
            body: JSON.stringify(ids),
        });
        return data ?? 0;
    }
}
