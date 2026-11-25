import {inject, injectable} from 'inversify';
import {API_BASE_URL} from '@/runtime-config.ts';
import {BaseServiceImpl} from './BaseServiceImpl.ts';
import {AgentService} from '@/service/AgentService.ts';
import {AuthServiceImpl} from '@/service/impl/AuthServiceImpl.ts';
import {REQ_WS_EVENT_AGENT} from '@/common/model/req/ReqWsEvent.ts';
import {ReqSocketContext} from '@/common/model/req/ReqWsContext.ts';
import {RespSocketContext} from '@/common/model/resp/RespWsContext.ts';
import {BaseApiServiceImpl} from '@/service/impl/BaseApiServiceImpl.ts';
import {isAgentEvent} from '@/common/model/resp/event/RespEventAgent.ts';
import type {RespAgentImageData} from '@/common/model/resp/RespAgent.ts';
import {WebSocketServiceImpl} from '@/service/impl/WebSocketServiceImpl.ts';
import {BizError, RESULT_CODE, type ResultModel, SUCCESS_CODE} from '@/common/model/resp/ResultModel.ts';
import type {AgentCancelEvent, AgentChatEvent, AgentConfirmModeEvent, AgentQuestionAnswerEvent, AgentSubscribeEvent, AgentToolAnswerEvent, AgentToolCancelEvent} from '@/common/model/req/event/WsAgent.ts';

const UNSUBSCRIBE_DELAY_MS = 300;

@injectable()
export class AgentServiceImpl extends BaseServiceImpl implements AgentService {
    private connectedOnce = false;
    private readonly subscribedChatIds: Set<string> = new Set();
    private readonly resyncListeners: Map<string, Set<() => void>> = new Map();
    private readonly pendingUnsubscribes: Map<string, ReturnType<typeof setTimeout>> = new Map();
    private readonly eventListeners: Map<string, Set<(arg: { context: RespSocketContext }) => void>> = new Map();

    constructor(
        @inject(AuthServiceImpl) private readonly authService: AuthServiceImpl,
        @inject(WebSocketServiceImpl) private readonly webSocketService: WebSocketServiceImpl,
    ) {
        super();
        this.webSocketService.addWsListener((context: RespSocketContext) => {
            this.handleRespContext(context);
        });
    }

    async start(): Promise<void> {
        this.webSocketService.addConnectListener(() => {
            this.subscribedChatIds.forEach((chatId) => this.sendSubscribe(chatId, REQ_WS_EVENT_AGENT.SUBSCRIBE));
            if (this.connectedOnce) {
                this.subscribedChatIds.forEach((chatId) => this.resyncListeners.get(chatId)?.forEach((callback) => callback()));
            }
            this.connectedOnce = true;
        });
        return super.start();
    }

    addEventListener(ai_chat_id: string, callback: (arg: { context: RespSocketContext }) => void): () => void {
        return this.authService.addSessionListener(this.eventListeners, ai_chat_id, callback);
    }

    addResyncListener(ai_chat_id: string, callback: () => void): () => void {
        return this.authService.addSessionListener(this.resyncListeners, ai_chat_id, callback);
    }

    handleSubscribe(ai_chat_id: string): void {
        const pending = this.pendingUnsubscribes.get(ai_chat_id);
        if (pending !== undefined) {
            clearTimeout(pending);
            this.pendingUnsubscribes.delete(ai_chat_id);
        }
        if (this.subscribedChatIds.has(ai_chat_id)) {
            return;
        }
        this.subscribedChatIds.add(ai_chat_id);
        this.sendSubscribe(ai_chat_id, REQ_WS_EVENT_AGENT.SUBSCRIBE);
    }

    handleUnsubscribe(ai_chat_id: string): void {
        if (!this.subscribedChatIds.has(ai_chat_id) || this.pendingUnsubscribes.has(ai_chat_id)) {
            return;
        }
        const timer = setTimeout(() => {
            this.pendingUnsubscribes.delete(ai_chat_id);
            this.subscribedChatIds.delete(ai_chat_id);
            this.sendSubscribe(ai_chat_id, REQ_WS_EVENT_AGENT.UNSUBSCRIBE);
        }, UNSUBSCRIBE_DELAY_MS);
        this.pendingUnsubscribes.set(ai_chat_id, timer);
    }

    handleChat(session_id: string, payload: AgentChatEvent): void {
        this.handleSubscribe(payload.data.aiChatId);
        const message: ReqSocketContext = {
            payload: payload,
            session_id: session_id,
            type: REQ_WS_EVENT_AGENT.CHAT,
        };
        this.webSocketService.send(message);
    }

    handleCancel(session_id: string, payload: AgentCancelEvent): void {
        const message: ReqSocketContext = {
            payload: payload,
            session_id: session_id,
            type: REQ_WS_EVENT_AGENT.CANCEL,
        };
        this.webSocketService.send(message);
    }

    handleToolAnswer(session_id: string, payload: AgentToolAnswerEvent): void {
        const message: ReqSocketContext = {
            payload: payload,
            session_id: session_id,
            type: REQ_WS_EVENT_AGENT.TOOL_ANSWER,
        };
        this.webSocketService.send(message);
    }

    handleConfirmMode(session_id: string, payload: AgentConfirmModeEvent): void {
        const message: ReqSocketContext = {
            payload: payload,
            session_id: session_id,
            type: REQ_WS_EVENT_AGENT.CONFIRM_MODE,
        };
        this.webSocketService.send(message);
    }

    handleQuestionAnswer(session_id: string, payload: AgentQuestionAnswerEvent): void {
        const message: ReqSocketContext = {
            payload: payload,
            session_id: session_id,
            type: REQ_WS_EVENT_AGENT.QUESTION_ANSWER,
        };
        this.webSocketService.send(message);
    }

    handleToolCancel(session_id: string, payload: AgentToolCancelEvent): void {
        const message: ReqSocketContext = {
            payload: payload,
            session_id: session_id,
            type: REQ_WS_EVENT_AGENT.TOOL_CANCEL,
        };
        this.webSocketService.send(message);
    }

    async readImage(path: string): Promise<RespAgentImageData> {
        const res = await fetch(`${API_BASE_URL}/ai_agent/image/read`, {
            method: 'POST',
            headers: {'Content-Type': 'application/json', ...BaseApiServiceImpl.buildAuthHeaders()},
            body: JSON.stringify({path}),
        });
        if (!res.ok) {
            throw new Error(`HTTP ${res.status}`);
        }
        const body = (await res.json()) as ResultModel<RespAgentImageData>;
        if (body.code !== SUCCESS_CODE) {
            throw new BizError(body.code);
        }
        if (!body.data) {
            throw new BizError(RESULT_CODE.CHAT_IMAGE_NOT_FOUND);
        }
        return body.data;
    }

    async saveImage(path: string, target: string): Promise<void> {
        const res = await fetch(`${API_BASE_URL}/ai_agent/image/save`, {
            method: 'POST',
            headers: {'Content-Type': 'application/json', ...BaseApiServiceImpl.buildAuthHeaders()},
            body: JSON.stringify({path, target}),
        });
        if (!res.ok) {
            throw new Error(`HTTP ${res.status}`);
        }
        const body = (await res.json()) as ResultModel<void>;
        if (body.code !== SUCCESS_CODE) {
            throw new BizError(body.code);
        }
    }

    handleRespContext(context: RespSocketContext): void {
        if (!isAgentEvent(context.type)) {
            return;
        }
        this.authService.emitEvent(this.eventListeners, context.session_id, {context});
    }

    private sendSubscribe(ai_chat_id: string, type: number): void {
        const message: ReqSocketContext = {
            type: type,
            session_id: ai_chat_id,
            payload: {data: {aiChatId: ai_chat_id}} satisfies AgentSubscribeEvent,
        };
        this.webSocketService.send(message);
    }

}
