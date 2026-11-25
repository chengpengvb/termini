import {inject, injectable} from 'inversify';
import {BaseApiServiceImpl} from './BaseApiServiceImpl';
import type {CodeIndexService} from '../CodeIndexService';
import {ENTITY_TYPE} from '@/common/const/EntityTypeConst.ts';
import type {IndexDirStatus} from '@/common/model/resp/RespCodeIndex.ts';
import {WebSocketServiceImpl} from '@/service/impl/WebSocketServiceImpl.ts';

@injectable()
export class CodeIndexServiceImpl extends BaseApiServiceImpl<IndexDirStatus> implements CodeIndexService {
    constructor(
        @inject(WebSocketServiceImpl) webSocketService: WebSocketServiceImpl,
    ) {
        super(webSocketService);
    }

    protected getNotifyEntityType(): number {
        return ENTITY_TYPE.NONE;
    }

    protected getPrefix(): string {
        return '/code_index';
    }

    async status(): Promise<IndexDirStatus[]> {
        const data = await this.request<IndexDirStatus[]>(this.buildUrl('/status'));
        if (!data) {
            return [];
        }
        return data;
    }

    async saveDirs(dirs: string[]): Promise<string[]> {
        const data = await this.request<string[]>(this.buildUrl('/save_dirs'), {
            method: 'POST',
            headers: this.JSON_HEADERS,
            body: JSON.stringify({dirs}),
        });
        if (!data) {
            return [];
        }
        return data;
    }

    async rebuild(dir: string): Promise<void> {
        await this.request<null>(this.buildUrl('/rebuild'), {
            method: 'POST',
            headers: this.JSON_HEADERS,
            body: JSON.stringify({dir}),
        });
    }
}
