import {inject, injectable} from 'inversify';
import {BaseApiServiceImpl} from './BaseApiServiceImpl';
import {ENTITY_TYPE} from '@/common/const/EntityTypeConst.ts';
import type {CommandRuleService} from '../CommandRuleService';
import {WebSocketServiceImpl} from '@/service/impl/WebSocketServiceImpl.ts';
import type {CommandRule, CreateCommandRuleReq} from '@/common/model/CommandRule.ts';

@injectable()
export class CommandRuleServiceImpl extends BaseApiServiceImpl<CommandRule> implements CommandRuleService {
    constructor(
        @inject(WebSocketServiceImpl) webSocketService: WebSocketServiceImpl,
    ) {
        super(webSocketService);
    }

    protected getNotifyEntityType(): number {
        return ENTITY_TYPE.COMMAND_RULE;
    }

    protected getPrefix(): string {
        return '/command_rule';
    }

    async create(req: CreateCommandRuleReq): Promise<CommandRule> {
        const data = await this.request<CommandRule>(this.buildUrl('/create'), {
            method: 'POST',
            headers: this.JSON_HEADERS,
            body: JSON.stringify(req),
        });
        if (!data) {
            throw new Error('create returned no data');
        }
        return data;
    }
}
