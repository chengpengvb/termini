import type {ApiService} from './ApiService';
import type {CommandRule, CreateCommandRuleReq} from '@/common/model/CommandRule.ts';

export interface CommandRuleService extends ApiService<CommandRule> {
    create(req: CreateCommandRuleReq): Promise<CommandRule>;
}
