import type { ApiService } from './ApiService';
import type { Device } from '../common/model/Device';
import { ReqDevice } from "@/common/model/req/ReqDevice.ts";
import type { RespTree } from '@/common/model/resp/RespTree.ts';

export interface DeviceService extends ApiService<Device> {

    ensureDeviceCache(): Promise<void>;

    selectAllWithConfig(): Promise<void>;

    localDevice(): Promise<ReqDevice | null>;

    selectDeviceTree(): Promise<RespTree[]>;

    getCachedDevice(id: string): Device | null;

    resolveDeviceLabel(id: string): string | null;

    delete(id: string | undefined): Promise<void>;

    saveOrUpdateWithConfig(req: ReqDevice): Promise<ReqDevice | null>;

    selectWithConfig(id: string | undefined): Promise<ReqDevice | null>;

    addDeviceListChangeListener(listener: (list: ReqDevice[]) => void): () => void;
}
