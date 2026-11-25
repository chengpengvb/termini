import {inject, injectable} from 'inversify';
import {BaseApiServiceImpl} from './BaseApiServiceImpl';
import {ENTITY_TYPE} from '@/common/const/EntityTypeConst.ts';
import type {AiFileChangeService} from '../AiFileChangeService';
import {WebSocketServiceImpl} from '@/service/impl/WebSocketServiceImpl.ts';
import type {RespSocketContext} from '@/common/model/resp/RespWsContext.ts';
import type {RespAiFileChangeView} from '@/common/model/resp/RespAiFileChange.ts';
import {isAiFileChangeEvent} from '@/common/model/resp/event/RespEventAiFileChange.ts';

@injectable()
export class AiFileChangeServiceImpl extends BaseApiServiceImpl<RespAiFileChangeView> implements AiFileChangeService {
    private readonly LISTENER_KEY = 'ai-file-change';

    constructor(
        @inject(WebSocketServiceImpl) private readonly webSocketService: WebSocketServiceImpl,
    ) {
        super(webSocketService);
        this.webSocketService.addWsListener((context: RespSocketContext) => {
            if (!isAiFileChangeEvent(context.type)) {
                return;
            }
            const chatId = context.payload as string;
            if (!chatId) {
                return;
            }
            this.notifyListeners<string>(this.LISTENER_KEY, chatId);
        });
    }

    protected getNotifyEntityType(): number {
        return ENTITY_TYPE.AI_FILE_CHANGE;
    }

    protected getPrefix(): string {
        return '/ai_file_change';
    }

    async selectViewByChatId(chatId: string): Promise<RespAiFileChangeView[]> {
        const data = await this.request<RespAiFileChangeView[]>(this.buildUrl(`/select_view_by_chat_id/${chatId}`));
        if (!data) {
            return [];
        }
        return data;
    }

    async accept(chatId: string, deviceId: string | null, path: string): Promise<void> {
        await this.request<unknown>(this.buildUrl('/accept'), {
            method: 'POST',
            headers: this.JSON_HEADERS,
            body: JSON.stringify({chatId, deviceId, path}),
        });
    }

    async revert(chatId: string, deviceId: string | null, path: string): Promise<void> {
        await this.request<unknown>(this.buildUrl('/revert'), {
            method: 'POST',
            headers: this.JSON_HEADERS,
            body: JSON.stringify({chatId, deviceId, path}),
        });
    }

    addFileChangeListener(listener: (chatId: string) => void): () => void {
        return this.addListener<string>(this.LISTENER_KEY, listener);
    }
}
