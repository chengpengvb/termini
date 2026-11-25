import {injectable} from 'inversify';
import type {SetService} from '../SetService';
import {BaseListenerService} from './BaseListenerService';
import {ENTITY_TYPE} from '@/common/const/EntityTypeConst.ts';
import {SET_MENU, SET_MENU_VALUES} from '@/page/common/model/SetMenu.ts';

@injectable()
export class SetServiceImpl extends BaseListenerService implements SetService {

    private selectedMenu: number = SET_MENU.SYSTEM;
    private readonly STORAGE_KEY_SELECTED_MENU = 'argus:set:selected-menu';

    constructor() {
        super();
        this.selectedMenu = this.loadSelectedMenu();
    }

    protected getNotifyEntityType(): number {
        return ENTITY_TYPE.NONE;
    }

    getSelectedMenu(): number {
        return this.selectedMenu;
    }

    setSelectedMenu(menu: number): void {
        if (!SET_MENU_VALUES.includes(menu)) {
            return;
        }
        this.selectedMenu = menu;
        window.localStorage.setItem(this.STORAGE_KEY_SELECTED_MENU, String(menu));
    }

    private loadSelectedMenu(): number {
        const raw = window.localStorage.getItem(this.STORAGE_KEY_SELECTED_MENU);
        if (!raw) {
            return SET_MENU.SYSTEM;
        }
        const parsed = Number(raw);
        if (!SET_MENU_VALUES.includes(parsed)) {
            return SET_MENU.SYSTEM;
        }
        return parsed;
    }
}
