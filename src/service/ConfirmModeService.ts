import type {BaseService} from './BaseService';

export interface ConfirmModeService extends BaseService {

    getConfirmMode(): number;

    setConfirmMode(mode: number): void;

    addConfirmModeListener(cb: (mode: number) => void): () => void;
}
