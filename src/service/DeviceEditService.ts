import type {BaseService} from './BaseService';
import type {ReqDevice} from '@/common/model/req/ReqDevice.ts';

export interface DeviceEditService extends BaseService {

    getSelectedAction(): number;

    setDraft(value: ReqDevice): void;

    clearDraft(deviceId: string): void;

    setSelectedAction(action: number): void;

    getDraft(deviceId: string): ReqDevice | null;

    addSelectedActionListener(cb: (action: number) => void): () => void;
}
