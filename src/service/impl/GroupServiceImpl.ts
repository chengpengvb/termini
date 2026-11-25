import {inject, injectable} from 'inversify';
import type {Group} from '@/common/model/Group';
import type {GroupService} from '../GroupService';
import {BaseApiServiceImpl} from './BaseApiServiceImpl';
import {ENTITY_TYPE} from '@/common/const/EntityTypeConst.ts';
import type {ReqGroupSet} from '@/common/model/req/ReqGroupSet';
import {WebSocketServiceImpl} from '@/service/impl/WebSocketServiceImpl.ts';

@injectable()
export class GroupServiceImpl extends BaseApiServiceImpl<Group> implements GroupService {
    constructor(
        @inject(WebSocketServiceImpl) webSocketService: WebSocketServiceImpl,
    ) {
        super(webSocketService);
    }

    protected getNotifyEntityType(): number {
        return ENTITY_TYPE.GROUP;
    }

    protected getPrefix(): string {
        return '/group';
    }

    async setDevice2Group(req: ReqGroupSet): Promise<void> {
        await this.request<unknown>(this.buildUrl('/set_device_2_group'), {
            method: 'POST',
            headers: this.JSON_HEADERS,
            body: JSON.stringify(req),
        });
    }
}
