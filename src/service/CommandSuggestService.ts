import type {BaseService} from './BaseService';
import type {ReqCommandSuggest} from '@/common/model/req/ReqCommandSuggest';

export interface CommandSuggestService extends BaseService {

    loadSshHistory(deviceId: string): Promise<string[]>;

    loadLocalHistory(shellId: string): Promise<string[]>;

    suggest(req: ReqCommandSuggest, signal: AbortSignal, onPartial: (list: string[]) => void): Promise<string[]>;

}
