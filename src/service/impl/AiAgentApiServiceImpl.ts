import {inject, injectable} from 'inversify';
import type {AiSkill} from '@/common/model/AiSkill.ts';
import {BaseApiServiceImpl} from './BaseApiServiceImpl';
import type {AiAgentApiService} from '../AiAgentApiService';
import {ENTITY_TYPE} from '@/common/const/EntityTypeConst.ts';
import {BizError, RESULT_CODE} from '@/common/model/resp/ResultModel.ts';
import {WebSocketServiceImpl} from '@/service/impl/WebSocketServiceImpl.ts';
import type {ReqShellDaemonStop} from '@/common/model/req/ReqShellDaemon.ts';
import type {RespAgentShellDaemonRow} from '@/common/model/resp/RespAgent.ts';
import type {ReqCancelGenerateSkill, ReqGenerateSkill} from '@/common/model/req/ReqGenerateSkill.ts';

@injectable()
export class AiAgentApiServiceImpl extends BaseApiServiceImpl implements AiAgentApiService {
    constructor(
        @inject(WebSocketServiceImpl) webSocketService: WebSocketServiceImpl,
    ) {
        super(webSocketService);
    }

    protected getNotifyEntityType(): number {
        return ENTITY_TYPE.NONE;
    }

    protected getPrefix(): string {
        return '/ai_agent';
    }

    async generateSkill(req: ReqGenerateSkill, signal?: AbortSignal): Promise<AiSkill> {
        const data = await this.request<AiSkill>(this.buildUrl('/generate_skill'), {
            method: 'POST',
            headers: this.JSON_HEADERS,
            body: JSON.stringify(req),
            signal,
        });
        if (!data) {
            throw new BizError(RESULT_CODE.SKILL_EXTRACT_EMPTY);
        }
        return data;
    }

    async cancelGenerateSkill(req: ReqCancelGenerateSkill): Promise<void> {
        await this.request<unknown>(this.buildUrl('/cancel_generate_skill'), {
            method: 'POST',
            headers: this.JSON_HEADERS,
            body: JSON.stringify(req),
        });
    }

    async listShellDaemons(chatId: string): Promise<RespAgentShellDaemonRow[]> {
        const data = await this.request<RespAgentShellDaemonRow[]>(this.buildUrl(`/shell_daemon/${chatId}`));
        if (!data) {
            return [];
        }
        return data;
    }

    async stopShellDaemon(req: ReqShellDaemonStop): Promise<void> {
        await this.request<unknown>(this.buildUrl('/shell_daemon/stop'), {
            method: 'POST',
            headers: this.JSON_HEADERS,
            body: JSON.stringify(req),
        });
    }
}
