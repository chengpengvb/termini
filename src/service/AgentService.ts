import type {BaseService} from './BaseService.ts';
import type {RespAgentImageData} from '@/common/model/resp/RespAgent.ts';
import type {RespSocketContext} from '@/common/model/resp/RespWsContext.ts';
import type {AgentCancelEvent, AgentChatEvent, AgentConfirmModeEvent, AgentQuestionAnswerEvent, AgentToolAnswerEvent, AgentToolCancelEvent} from '@/common/model/req/event/WsAgent.ts';

export interface AgentService extends BaseService {
    handleSubscribe(ai_chat_id: string): void;

    handleUnsubscribe(ai_chat_id: string): void;

    readImage(path: string): Promise<RespAgentImageData>;

    saveImage(path: string, target: string): Promise<void>;

    handleChat(session_id: string, payload: AgentChatEvent): void;

    handleCancel(session_id: string, payload: AgentCancelEvent): void;

    addResyncListener(ai_chat_id: string, callback: () => void): () => void;

    handleToolAnswer(session_id: string, payload: AgentToolAnswerEvent): void;

    handleToolCancel(session_id: string, payload: AgentToolCancelEvent): void;

    handleConfirmMode(session_id: string, payload: AgentConfirmModeEvent): void;

    handleQuestionAnswer(session_id: string, payload: AgentQuestionAnswerEvent): void;

    addEventListener(ai_chat_id: string, callback: (arg: { context: RespSocketContext }) => void): () => void;
}
