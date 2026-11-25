import {inject, injectable} from 'inversify';
import type {AiPlanService} from '../AiPlanService.ts';
import {BaseApiServiceImpl} from './BaseApiServiceImpl.ts';
import {ENTITY_TYPE} from '@/common/const/EntityTypeConst.ts';
import type {AiPlanRun, AiPlanTask} from '@/common/model/AiPlan.ts';
import {WebSocketServiceImpl} from '@/service/impl/WebSocketServiceImpl.ts';

@injectable()
export class AiPlanServiceImpl extends BaseApiServiceImpl<AiPlanTask> implements AiPlanService {
    constructor(
        @inject(WebSocketServiceImpl) webSocketService: WebSocketServiceImpl,
    ) {
        super(webSocketService);
    }

    protected getNotifyEntityType(): number {
        return ENTITY_TYPE.AI_PLAN_TASK;
    }

    protected getPrefix(): string {
        return '/ai_plan_task';
    }

    async save(task: AiPlanTask): Promise<AiPlanTask> {
        const data = await this.request<AiPlanTask>(this.buildUrl('/save'), {
            method: 'POST',
            headers: this.JSON_HEADERS,
            body: JSON.stringify(task),
        });
        if (data === null) {
            throw new Error('plan task save returned no data');
        }
        return data;
    }

    async deleteWithRuns(id: string): Promise<void> {
        await this.request<boolean>(this.buildUrl(`/delete_with_runs/${id}`), {method: 'DELETE'});
    }

    async cancelRun(runId: string): Promise<void> {
        await this.request<void>(this.buildUrl(`/cancel_run/${runId}`), {method: 'POST'});
    }

    async deleteRun(runId: string): Promise<void> {
        await this.request<void>(this.buildUrl(`/delete_run/${runId}`), {method: 'DELETE'});
    }

    async runNow(id: string): Promise<AiPlanRun> {
        const data = await this.request<AiPlanRun>(this.buildUrl(`/run_now/${id}`), {method: 'POST'});
        if (data === null) {
            throw new Error('plan task run returned no data');
        }
        return data;
    }

    async runs(): Promise<AiPlanRun[]> {
        const data = await this.request<AiPlanRun[]>(this.buildUrl('/runs'));
        if (!data) {
            return [];
        }
        return data;
    }
}
