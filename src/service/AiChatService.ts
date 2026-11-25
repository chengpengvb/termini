import type {ApiService} from './ApiService';
import type {AiChat} from '@/common/model/AiChat.ts';
import type {ChatInputDraft} from '@/common/model/dto/ChatInputDraft.ts';
import type {QueuedChatMessage} from '@/common/model/dto/QueuedChatMessage.ts';

export interface AiChatService extends ApiService<AiChat> {
    touch(id: string): Promise<void>;

    clearInputCache(chatId: string | null): void;

    rename(id: string, title: string): Promise<void>;

    getMessageQueue(chatId: string): QueuedChatMessage[];

    revert(chatId: string, itemId: string): Promise<void>;

    getInputCache(chatId: string | null): ChatInputDraft | null;

    setMessageQueue(chatId: string, list: QueuedChatMessage[]): void;

    setInputCache(chatId: string | null, draft: ChatInputDraft): void;

    addAiChatListChangeListener(listener: (list: AiChat[]) => void): () => void;
}
