import type {ApiService} from './ApiService';
import type {AiChatItem} from '@/common/model/AiChatItem.ts';
import type {ContextUsage} from '@/common/model/dto/ContextUsage.ts';

export interface AiChatItemService extends ApiService<AiChatItem> {
    isRunning(chatId: string): Promise<string | null>;

    searchChatIds(keyword: string): Promise<string[]>;

    selectByChatId(chatId: string): Promise<AiChatItem[]>;

    contextUsage(chatId: string, provideId: string | null): Promise<ContextUsage | null>;
}
