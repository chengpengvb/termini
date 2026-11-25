import {injectable} from 'inversify';
import {BaseServiceImpl} from './BaseServiceImpl';
import type {AiShareConsentService} from '../AiShareConsentService';

@injectable()
export class AiShareConsentServiceImpl extends BaseServiceImpl implements AiShareConsentService {

    private readonly DONE_VALUE = '1';
    private readonly STORAGE_KEY = 'argus:agent:aiShareConsent:done';

    selectConsentDone(): boolean {
        const raw = window.localStorage.getItem(this.STORAGE_KEY);
        if (raw === null) {
            return false;
        }
        if (raw === this.DONE_VALUE) {
            return true;
        }
        throw new Error(`argus:agent:aiShareConsent:done invalid: ${raw}`);
    }

    saveConsentDone(): void {
        window.localStorage.setItem(this.STORAGE_KEY, this.DONE_VALUE);
    }
}
