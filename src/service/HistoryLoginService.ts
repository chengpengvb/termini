import type {BaseService} from './BaseService';
import type {HistoryLogin} from '@/common/model/dto/HistoryLogin.ts';

export interface HistoryLoginService extends BaseService {

    clear(): void;

    list(): HistoryLogin[];

    removeByDeviceId(deviceId: string): void;

    getLastProtocols(deviceId: string): number[];

    add(deviceId: string, protocol: number, time?: number): void;

    addListChangeListener(listener: (list: HistoryLogin[]) => void): () => void;
}
