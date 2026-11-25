import {inject, injectable} from 'inversify';
import {BaseApiServiceImpl} from './BaseApiServiceImpl';
import {ENTITY_TYPE} from '@/common/const/EntityTypeConst.ts';
import type {DeviceImportService} from '../DeviceImportService';
import {WebSocketServiceImpl} from '@/service/impl/WebSocketServiceImpl.ts';
import type {DeviceImportApplyResult, DeviceImportItem} from '@/common/model/DeviceImport.ts';

@injectable()
export class DeviceImportServiceImpl extends BaseApiServiceImpl implements DeviceImportService {
    constructor(
        @inject(WebSocketServiceImpl) webSocketService: WebSocketServiceImpl,
    ) {
        super(webSocketService);
    }

    protected getNotifyEntityType(): number {
        return ENTITY_TYPE.NONE;
    }

    protected getPrefix(): string {
        return '/device_import';
    }

    async preview(source: number, content: string): Promise<DeviceImportItem[]> {
        const data = await this.request<DeviceImportItem[]>(this.buildUrl('/preview'), {
            method: 'POST',
            headers: this.JSON_HEADERS,
            body: JSON.stringify({source, content}),
        });
        if (!data) {
            return [];
        }
        return data;
    }

    async apply(items: DeviceImportItem[]): Promise<DeviceImportApplyResult> {
        const data = await this.request<DeviceImportApplyResult>(this.buildUrl('/apply'), {
            method: 'POST',
            headers: this.JSON_HEADERS,
            body: JSON.stringify({items}),
        });
        if (!data) {
            throw new Error('apply returned no data');
        }
        return data;
    }
}
