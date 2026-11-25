import type {ApiService} from './ApiService';
import type {ConfigKeyPair} from '@/common/model/ConfigKeyPair';

export interface ConfigKeyPairService extends ApiService<ConfigKeyPair> {
    reloadAll(): Promise<void>;
    addKeyPairListChangeListener(listener: (list: ConfigKeyPair[]) => void): () => void;
}
