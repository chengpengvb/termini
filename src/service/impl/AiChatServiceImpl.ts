import {inject, injectable} from 'inversify';
import type {JSONContent} from '@tiptap/core';
import type {AiChatService} from '../AiChatService';
import {BaseApiServiceImpl} from './BaseApiServiceImpl';
import {ENTITY_TYPE} from '@/common/const/EntityTypeConst.ts';
import {AI_CHAT_TYPE, type AiChat} from '@/common/model/AiChat.ts';
import type {ChatInputDraft} from '@/common/model/dto/ChatInputDraft.ts';
import {isAiChatEvent} from '@/common/model/resp/event/RespEventAiChat.ts';
import {WebSocketServiceImpl} from '@/service/impl/WebSocketServiceImpl.ts';
import type {RespSocketContext} from '@/common/model/resp/RespWsContext.ts';
import type {QueuedChatMessage} from '@/common/model/dto/QueuedChatMessage.ts';

@injectable()
export class AiChatServiceImpl extends BaseApiServiceImpl<AiChat> implements AiChatService {
    private aiChatList: AiChat[] = [];
    private readonly LISTENER_KEY = 'ai-chat-list-change';
    private readonly STORAGE_KEY_INPUT_CACHE = 'argus:agent:input-cache';
    private readonly STORAGE_KEY_MESSAGE_QUEUE = 'argus:agent:message-queue';
    private readonly messageQueues: Map<string, QueuedChatMessage[]> = new Map();

    constructor(
        @inject(WebSocketServiceImpl) private readonly webSocketService: WebSocketServiceImpl,
    ) {
        super(webSocketService);
        this.webSocketService.addWsListener((context: RespSocketContext) => {
            if (!isAiChatEvent(context.type)) {
                return;
            }
            void this.refresh();
        });
    }

    protected getNotifyEntityType(): number {
        return ENTITY_TYPE.AI_CHAT;
    }

    async start(): Promise<void> {
        void this.refresh();
        return super.start();
    }

    protected override onEntityRefresh(): void {
        void this.refresh();
    }

    protected getPrefix(): string {
        return '/ai_chat';
    }

    addAiChatListChangeListener(listener: (list: AiChat[]) => void): () => void {
        return this.addListener<AiChat[]>(this.LISTENER_KEY, listener);
    }

    async touch(id: string): Promise<void> {
        await this.request<unknown>(this.buildUrl(`/touch/${id}`), {method: 'POST'});
        await this.refresh();
    }

    async rename(id: string, title: string): Promise<void> {
        await this.request<unknown>(this.buildUrl('/rename'), {
            method: 'POST',
            headers: this.JSON_HEADERS,
            body: JSON.stringify({id, title}),
        });
        await this.refresh();
    }

    async revert(chatId: string, itemId: string): Promise<void> {
        await this.request<unknown>(this.buildUrl('/revert'), {
            method: 'POST',
            headers: this.JSON_HEADERS,
            body: JSON.stringify({chatId, itemId}),
        });
    }

    override async selectAll(): Promise<AiChat[]> {
        const data = await this.request<AiChat[]>(this.buildUrl(`/select_by_type/${AI_CHAT_TYPE.CHAT}`));
        if (!data) {
            this.aiChatList = [];
            return this.aiChatList;
        }
        this.aiChatList = data;
        return this.aiChatList;
    }

    override async insert(item: AiChat): Promise<AiChat> {
        const saved = await super.insert(item);
        await this.refresh();
        return saved;
    }

    override async update(item: AiChat): Promise<AiChat> {
        const saved = await super.update(item);
        await this.refresh();
        return saved;
    }

    override async deleteById(id: string): Promise<void> {
        await super.deleteById(id);
        this.clearInputCache(id);
        this.setMessageQueue(id, []);
        await this.refresh();
    }

    getInputCache(chatId: string | null): ChatInputDraft | null {
        const raw = window.localStorage.getItem(this.resolveInputCacheKey(chatId));
        if (raw === null) {
            return null;
        }
        const parsed = JSON.parse(raw) as JSONContent;
        if (parsed.type === 'doc') {
            return {content: parsed};
        }
        return parsed as ChatInputDraft;
    }

    setInputCache(chatId: string | null, draft: ChatInputDraft): void {
        if (draft.content === null && draft.revert === undefined) {
            this.clearInputCache(chatId);
            return;
        }
        window.localStorage.setItem(this.resolveInputCacheKey(chatId), JSON.stringify(draft));
    }

    clearInputCache(chatId: string | null): void {
        window.localStorage.removeItem(this.resolveInputCacheKey(chatId));
    }

    getMessageQueue(chatId: string): QueuedChatMessage[] {
        const list = this.messageQueues.get(chatId);
        if (list !== undefined) {
            return list;
        }
        const raw = window.localStorage.getItem(this.resolveMessageQueueKey(chatId));
        if (raw === null) {
            return [];
        }
        const restored = JSON.parse(raw) as QueuedChatMessage[];
        this.messageQueues.set(chatId, restored);
        return restored;
    }

    setMessageQueue(chatId: string, list: QueuedChatMessage[]): void {
        if (list.length === 0) {
            this.messageQueues.delete(chatId);
            window.localStorage.removeItem(this.resolveMessageQueueKey(chatId));
            return;
        }
        this.messageQueues.set(chatId, list);
        const durable = list.filter(item => item.images.length === 0);
        if (durable.length === 0) {
            window.localStorage.removeItem(this.resolveMessageQueueKey(chatId));
            return;
        }
        window.localStorage.setItem(this.resolveMessageQueueKey(chatId), JSON.stringify(durable));
    }

    private resolveInputCacheKey(chatId: string | null): string {
        if (chatId === null) {
            return this.STORAGE_KEY_INPUT_CACHE;
        }
        return `${this.STORAGE_KEY_INPUT_CACHE}:${chatId}`;
    }

    private resolveMessageQueueKey(chatId: string): string {
        return `${this.STORAGE_KEY_MESSAGE_QUEUE}:${chatId}`;
    }

    private async refresh(): Promise<void> {
        await this.selectAll();
        this.notify();
    }

    private notify(): void {
        this.notifyListeners<AiChat[]>(this.LISTENER_KEY, this.aiChatList);
    }
}
