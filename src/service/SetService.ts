import type {BaseService} from './BaseService';

export interface SetService extends BaseService {

    getSelectedMenu(): number;

    setSelectedMenu(menu: number): void;
}
