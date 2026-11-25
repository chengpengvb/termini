import type {ApiService} from './ApiService';
import type {RespAiFileChangeView} from '@/common/model/resp/RespAiFileChange.ts';

export interface AiFileChangeService extends ApiService<RespAiFileChangeView> {
    selectViewByChatId(chatId: string): Promise<RespAiFileChangeView[]>;

    addFileChangeListener(listener: (chatId: string) => void): () => void;

    accept(chatId: string, deviceId: string | null, path: string): Promise<void>;

    revert(chatId: string, deviceId: string | null, path: string): Promise<void>;

}
