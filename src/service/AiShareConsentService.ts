import type {BaseService} from './BaseService';

export interface AiShareConsentService extends BaseService {

    selectConsentDone(): boolean;

    saveConsentDone(): void;
}
