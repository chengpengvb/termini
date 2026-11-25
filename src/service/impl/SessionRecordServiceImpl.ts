import {inject, injectable} from 'inversify';
import {BaseApiServiceImpl} from './BaseApiServiceImpl.ts';
import {ENTITY_TYPE} from '@/common/const/EntityTypeConst.ts';
import type {SessionRecord} from '@/common/model/SessionRecord.ts';
import type {SessionRecordService} from '../SessionRecordService.ts';
import {WebSocketServiceImpl} from '@/service/impl/WebSocketServiceImpl.ts';

@injectable()
export class SessionRecordServiceImpl extends BaseApiServiceImpl<SessionRecord> implements SessionRecordService {
    constructor(
        @inject(WebSocketServiceImpl) webSocketService: WebSocketServiceImpl,
    ) {
        super(webSocketService);
    }

    protected getNotifyEntityType(): number {
        return ENTITY_TYPE.SESSION_RECORD;
    }

    protected getPrefix(): string {
        return '/session_record';
    }

    async list(): Promise<SessionRecord[]> {
        const data = await this.request<SessionRecord[]>(this.buildUrl('/list'));
        if (!data) {
            return [];
        }
        return data;
    }

    async content(id: string): Promise<string> {
        const data = await this.request<string>(this.buildUrl(`/content/${id}`));
        if (data === null) {
            throw new Error('session record content missing');
        }
        return data;
    }

    async deleteBefore(startedBeforeMillis: number): Promise<number> {
        const data = await this.request<number>(this.buildUrl('/delete_before'), {
            method: 'POST',
            headers: this.JSON_HEADERS,
            body: JSON.stringify({startedBeforeMillis}),
        });
        return data ?? 0;
    }
}
