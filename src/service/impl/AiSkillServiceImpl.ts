import {inject, injectable} from 'inversify';
import type {AiSkillService} from '../AiSkillService';
import type {AiSkill} from '@/common/model/AiSkill.ts';
import {BaseApiServiceImpl} from './BaseApiServiceImpl';
import {ENTITY_TYPE} from '@/common/const/EntityTypeConst.ts';
import {WebSocketServiceImpl} from '@/service/impl/WebSocketServiceImpl.ts';

@injectable()
export class AiSkillServiceImpl extends BaseApiServiceImpl<AiSkill> implements AiSkillService {
    constructor(
        @inject(WebSocketServiceImpl) webSocketService: WebSocketServiceImpl,
    ) {
        super(webSocketService);
    }

    protected getNotifyEntityType(): number {
        return ENTITY_TYPE.AI_SKILL;
    }

    protected getPrefix(): string {
        return '/ai_skill';
    }
}
