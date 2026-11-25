import {inject, injectable} from 'inversify';
import {API_BASE_URL} from '@/runtime-config.ts';
import {BaseListenerService} from './BaseListenerService';
import {ENTITY_TYPE} from '@/common/const/EntityTypeConst.ts';
import type {AiChat2ProjectService} from '../AiChat2ProjectService';
import type {ResultModel} from '@/common/model/resp/ResultModel.ts';
import type {AiChat2Project} from '@/common/model/AiChat2Project.ts';
import {BaseApiServiceImpl} from '@/service/impl/BaseApiServiceImpl.ts';
import {BizError, SUCCESS_CODE} from '@/common/model/resp/ResultModel.ts';
import {WebSocketServiceImpl} from '@/service/impl/WebSocketServiceImpl.ts';
import type {RespSocketContext} from '@/common/model/resp/RespWsContext.ts';
import {isAiProjectEvent} from '@/common/model/resp/event/RespEventAiProject.ts';
import {isEntityNotifyEvent} from '@/common/model/resp/event/RespEventEntityNotify.ts';

@injectable()
export class AiChat2ProjectServiceImpl extends BaseListenerService implements AiChat2ProjectService {
    private linkList: AiChat2Project[] = [];
    private linkMap: Map<string, string> = new Map();
    private readonly LISTENER_KEY = 'ai-chat-2-project-list-change';
    private readonly JSON_HEADERS: Record<string, string> = {'Content-Type': 'application/json'};

    constructor(
        @inject(WebSocketServiceImpl) private readonly webSocketService: WebSocketServiceImpl,
    ) {
        super();
        this.webSocketService.addWsListener((context: RespSocketContext) => {
            if (isEntityNotifyEvent(context.type)) {
                this.handleEntityNotifyPayload(context.payload);
                return;
            }
            if (!isAiProjectEvent(context.type)) {
                return;
            }
            void this.refresh();
        });
    }

    protected getNotifyEntityType(): number {
        return ENTITY_TYPE.AI_CHAT_2_PROJECT;
    }

    async start(): Promise<void> {
        void this.refresh();
        return super.start();
    }

    protected override onEntityRefresh(): void {
        void this.refresh();
    }

    async list(): Promise<AiChat2Project[]> {
        const data = await this.request<AiChat2Project[]>(`${API_BASE_URL}/ai_chat_2_project/list`);
        if (!data) {
            this.linkList = [];
            this.linkMap = new Map();
            return this.linkList;
        }
        this.linkList = data;
        this.linkMap = new Map(data.map((item) => [item.chat_id, item.project_id]));
        return this.linkList;
    }

    async setProject(chatId: string, projectId: string): Promise<void> {
        await this.request<unknown>(`${API_BASE_URL}/ai_chat_2_project/set`, {
            method: 'POST',
            headers: this.JSON_HEADERS,
            body: JSON.stringify({chatId, projectId}),
        });
        await this.refresh();
    }

    async unset(chatId: string): Promise<void> {
        await this.request<unknown>(`${API_BASE_URL}/ai_chat_2_project/unset/${chatId}`, {method: 'POST'});
        await this.refresh();
    }

    resolveProjectId(chatId: string): string | null {
        return this.linkMap.get(chatId) ?? null;
    }

    addAiChat2ProjectListChangeListener(listener: (list: AiChat2Project[]) => void): () => void {
        return this.addListener<AiChat2Project[]>(this.LISTENER_KEY, listener);
    }

    private async refresh(): Promise<void> {
        await this.list();
        this.notifyListeners<AiChat2Project[]>(this.LISTENER_KEY, this.linkList);
    }

    private async request<R>(url: string, init?: RequestInit): Promise<R | null> {
        const res = await fetch(url, {
            ...init,
            headers: {...BaseApiServiceImpl.buildAuthHeaders(), ...(init?.headers as Record<string, string>)},
        });
        if (!res.ok) {
            const text = await res.text();
            throw new Error(`HTTP ${res.status} ${url}: ${text}`);
        }
        const body = (await res.json()) as ResultModel<R>;
        if (body.code !== SUCCESS_CODE) {
            throw new BizError(body.code);
        }
        return body.data ?? null;
    }
}
