import {injectable} from 'inversify';
import {BaseListenerService} from './BaseListenerService';
import type {DeviceEditService} from '../DeviceEditService';
import {ENTITY_TYPE} from '@/common/const/EntityTypeConst.ts';
import type {ReqDevice} from '@/common/model/req/ReqDevice.ts';
import {DEVICE_EDIT_ACTION_CONNECT, DEVICE_EDIT_ACTION_SAVE} from '@/common/model/dto/DeviceEditAction.ts';

@injectable()
export class DeviceEditServiceImpl extends BaseListenerService implements DeviceEditService {

    private readonly KEY_SELECTED_ACTION = 'selected-action';
    private readonly drafts = new Map<string, ReqDevice>();
    private selectedAction: number = DEVICE_EDIT_ACTION_CONNECT;
    private readonly STORAGE_KEY_SELECTED_ACTION = 'argus:device-edit:selected-action';

    constructor() {
        super();
        this.selectedAction = this.loadSelectedAction();
    }

    protected getNotifyEntityType(): number {
        return ENTITY_TYPE.NONE;
    }

    getSelectedAction(): number {
        return this.selectedAction;
    }

    setSelectedAction(action: number): void {
        if (action !== DEVICE_EDIT_ACTION_SAVE && action !== DEVICE_EDIT_ACTION_CONNECT) {
            return;
        }
        window.localStorage.setItem(this.STORAGE_KEY_SELECTED_ACTION, String(action));
        if (this.selectedAction === action) {
            return;
        }
        this.selectedAction = action;
        this.notifyListeners<number>(this.KEY_SELECTED_ACTION, action);
    }

    addSelectedActionListener(cb: (action: number) => void): () => void {
        return this.addListener<number>(this.KEY_SELECTED_ACTION, cb);
    }

    getDraft(deviceId: string): ReqDevice | null {
        const draft = this.drafts.get(deviceId);
        if (draft === undefined) {
            return null;
        }
        return draft;
    }

    setDraft(value: ReqDevice): void {
        this.drafts.set(value.device.id, value);
    }

    clearDraft(deviceId: string): void {
        this.drafts.delete(deviceId);
    }


    private loadSelectedAction(): number {
        const raw = window.localStorage.getItem(this.STORAGE_KEY_SELECTED_ACTION);
        if (!raw) {
            return DEVICE_EDIT_ACTION_CONNECT;
        }
        const parsed = Number(raw);
        if (parsed === DEVICE_EDIT_ACTION_SAVE) {
            return DEVICE_EDIT_ACTION_SAVE;
        }
        if (parsed === DEVICE_EDIT_ACTION_CONNECT) {
            return DEVICE_EDIT_ACTION_CONNECT;
        }
        return DEVICE_EDIT_ACTION_CONNECT;
    }
}
