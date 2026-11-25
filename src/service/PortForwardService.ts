import type {ApiService} from './ApiService';
import type {PortForward} from '@/common/model/PortForward';

export interface PortForwardService extends ApiService<PortForward> {
    reloadAll(): Promise<void>;
    selectByDevice(deviceId: string): Promise<PortForward[]>;
    addForwardListChangeListener(listener: (list: PortForward[]) => void): () => void;
}
