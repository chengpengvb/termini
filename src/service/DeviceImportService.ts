import type {BaseService} from './BaseService';
import type {DeviceImportApplyResult, DeviceImportItem} from '@/common/model/DeviceImport.ts';

export interface DeviceImportService extends BaseService {
    preview(source: number, content: string): Promise<DeviceImportItem[]>;

    apply(items: DeviceImportItem[]): Promise<DeviceImportApplyResult>;
}
