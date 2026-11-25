import {inject, injectable} from 'inversify';
import {BaseApiServiceImpl} from './BaseApiServiceImpl';
import type {AiChatItemService} from '../AiChatItemService';
import type {AiChatItem} from '@/common/model/AiChatItem.ts';
import {ENTITY_TYPE} from '@/common/const/EntityTypeConst.ts';
import type {ContextUsage} from '@/common/model/dto/ContextUsage.ts';
import {WebSocketServiceImpl} from '@/service/impl/WebSocketServiceImpl.ts';

@injectable()
export class AiChatItemServiceImpl extends BaseApiServiceImpl<AiChatItem> implements AiChatItemService {
    constructor(
        @inject(WebSocketServiceImpl) webSocketService: WebSocketServiceImpl,
    ) {
        super(webSocketService);
    }

    protected getNotifyEntityType(): number {
        return ENTITY_TYPE.AI_CHAT_ITEM;
    }

    protected getPrefix(): string {
        return '/ai_chat_item';
    }

    async selectByChatId(chatId: string): Promise<AiChatItem[]> {
        const data = await this.request<AiChatItem[]>(this.buildUrl(`/select_by_chat_id/${chatId}`));
        if (!data) {
            return [];
        }
        return data;
    }

    async searchChatIds(keyword: string): Promise<string[]> {
        const data = await this.request<string[]>(this.buildUrl(`/search_chat_ids?q=${encodeURIComponent(keyword)}`));
        return data ?? [];
    }

    async isRunning(chatId: string): Promise<string | null> {
        return this.request<string>(this.buildUrl(`/is_running/${chatId}`));
    }

    async contextUsage(chatId: string, provideId: string | null): Promise<ContextUsage | null> {
        const provide = provideId && provideId.length > 0 ? encodeURIComponent(provideId) : '-';
        return this.request<ContextUsage>(this.buildUrl(`/context_usage/${chatId}/${provide}`));
    }
}
