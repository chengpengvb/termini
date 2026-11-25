import {inject, injectable} from 'inversify';
import {BaseApiServiceImpl} from './BaseApiServiceImpl';
import type {FileEditService} from '../FileEditService';
import {ENTITY_TYPE} from '@/common/const/EntityTypeConst.ts';
import type {RespFileEditRead} from '@/common/model/resp/RespFileEdit.ts';
import {WebSocketServiceImpl} from '@/service/impl/WebSocketServiceImpl.ts';

@injectable()
export class FileEditServiceImpl extends BaseApiServiceImpl<RespFileEditRead> implements FileEditService {
    constructor(
        @inject(WebSocketServiceImpl) webSocketService: WebSocketServiceImpl,
    ) {
        super(webSocketService);
    }

    protected getNotifyEntityType(): number {
        return ENTITY_TYPE.NONE;
    }

    protected getPrefix(): string {
        return '/file_edit';
    }

    async read(path: string): Promise<RespFileEditRead> {
        const data = await this.request<RespFileEditRead>(this.buildUrl('/read'), {
            method: 'POST',
            headers: this.JSON_HEADERS,
            body: JSON.stringify({path}),
        });
        if (!data) {
            throw new Error(`read returned no data: ${path}`);
        }
        return data;
    }

    async write(path: string, content: string, create: boolean = false): Promise<void> {
        await this.request<unknown>(this.buildUrl('/write'), {
            method: 'POST',
            headers: this.JSON_HEADERS,
            body: JSON.stringify({path, content, create}),
        });
    }
}
