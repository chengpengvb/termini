import {injectable} from 'inversify';
import {BaseListenerService} from './BaseListenerService';
import {ENTITY_TYPE} from '@/common/const/EntityTypeConst.ts';
import type {ConfirmModeService} from '../ConfirmModeService';
import {CONFIRM_MODE_AUTO, CONFIRM_MODE_MANUAL, CONFIRM_MODE_SILENT} from '@/common/model/dto/ConfirmMode.ts';

@injectable()
export class ConfirmModeServiceImpl extends BaseListenerService implements ConfirmModeService {

    private readonly KEY_CONFIRM_MODE = 'confirm-mode';
    private confirmMode: number = CONFIRM_MODE_MANUAL;
    private readonly STORAGE_KEY_CONFIRM_MODE = 'argus:agent:confirm-mode';

    constructor() {
        super();
        this.confirmMode = this.loadConfirmMode();
    }

    protected getNotifyEntityType(): number {
        return ENTITY_TYPE.NONE;
    }

    getConfirmMode(): number {
        return this.confirmMode;
    }

    setConfirmMode(mode: number): void {
        if (this.resolveConfirmMode(mode) !== mode) {
            return;
        }
        window.localStorage.setItem(this.STORAGE_KEY_CONFIRM_MODE, String(mode));
        if (this.confirmMode === mode) {
            return;
        }
        this.confirmMode = mode;
        this.notifyListeners<number>(this.KEY_CONFIRM_MODE, mode);
    }

    addConfirmModeListener(cb: (mode: number) => void): () => void {
        return this.addListener<number>(this.KEY_CONFIRM_MODE, cb);
    }

    private loadConfirmMode(): number {
        const raw = window.localStorage.getItem(this.STORAGE_KEY_CONFIRM_MODE);
        if (!raw) {
            return CONFIRM_MODE_MANUAL;
        }
        return this.resolveConfirmMode(Number(raw));
    }

    private resolveConfirmMode(mode: number): number {
        if (mode === CONFIRM_MODE_AUTO || mode === CONFIRM_MODE_SILENT) {
            return mode;
        }
        return CONFIRM_MODE_MANUAL;
    }
}
