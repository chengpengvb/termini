import {RefObject, useEffect, useRef, useState} from 'react';
import type {AiChatItem} from '@/common/model/AiChatItem.ts';
import type {AgentCancelEvent} from '@/common/model/req/event/WsAgent.ts';
import type {RespSocketContext} from '@/common/model/resp/RespWsContext.ts';
import {RESP_EVENT_AGENT} from '@/common/model/resp/event/RespEventAgent.ts';
import {useAgentService, useAiChatItemService} from '@/common/context/useAppHooks.tsx';

export interface UseAgentRunParams {
    sessionId: string;
    chatId: string | null;
    onRunAborted: () => void;
    chatIdRef: RefObject<string | null>;
    onRunDone: (questionPending: boolean) => void;
}

export function useAgentRun(params: UseAgentRunParams) {
    const agentService = useAgentService();
    const runFailedRef = useRef<boolean>(false);
    const {chatId, sessionId, chatIdRef} = params;
    const aiChatItemService = useAiChatItemService();
    const activeReqIdRef = useRef<string | null>(null);
    const questionPendingRef = useRef<boolean>(false);
    const paramsRef = useRef<UseAgentRunParams>(params);
    const [running, setRunning] = useState<boolean>(false);

    paramsRef.current = params;

    function clearRunning(): void {
        activeReqIdRef.current = null;
        runFailedRef.current = false;
        questionPendingRef.current = false;
        setRunning(false);
    }

    function canAcceptAgentItem(item: AiChatItem): boolean {
        const currentChatId = chatIdRef.current;
        if (!currentChatId) {
            return false;
        }
        if (!item.chat_id) {
            return false;
        }
        return item.chat_id === currentChatId;
    }

    function handleStop(): void {
        const reqId = activeReqIdRef.current;
        if (!reqId) {
            throw new Error('agent stop missing reqId');
        }
        const event: AgentCancelEvent = {
            data: {reqId},
        };
        agentService.handleCancel(sessionId, event);
    }

    function handleAgentItem(item: AiChatItem): void {
        if (!canAcceptAgentItem(item)) {
            return;
        }
        if (item.context?.parentReqId) {
            return;
        }
        const reqId = item.context?.reqId ?? null;
        if (item.type === RESP_EVENT_AGENT.ON_DONE || item.type === RESP_EVENT_AGENT.ON_CANCELLED) {
            if (!reqId || activeReqIdRef.current !== reqId) {
                return;
            }
            const failed = runFailedRef.current;
            const questionPending = questionPendingRef.current;
            clearRunning();
            if (item.type === RESP_EVENT_AGENT.ON_DONE && !failed) {
                paramsRef.current.onRunDone(questionPending);
                return;
            }
            paramsRef.current.onRunAborted();
            return;
        }
        if (!reqId) {
            return;
        }
        // 任何非终止事件都表明该 run 正在进行: 据此采纳 running 与 reqId。
        // 不再只认一次性的 ON_OPEN——新会话首条消息切换 chatId 时可能漏收 ON_OPEN，
        // 导致按钮停留在发送态而非取消态；用后续事件兜底即可稳健对齐。
        const isNewRun = activeReqIdRef.current !== reqId;
        if (isNewRun || item.type === RESP_EVENT_AGENT.ON_OPEN) {
            runFailedRef.current = false;
            questionPendingRef.current = false;
        }
        if (item.type === RESP_EVENT_AGENT.ON_ERROR) {
            runFailedRef.current = true;
        }
        if (item.type === RESP_EVENT_AGENT.ON_QUESTION) {
            questionPendingRef.current = true;
        }
        activeReqIdRef.current = reqId;
        setRunning(true);
    }

    useEffect(() => {
        clearRunning();
        if (!chatId) {
            return;
        }
        let disposed = false;
        aiChatItemService.isRunning(chatId).then((reqId) => {
            if (disposed) {
                return;
            }
            if (!reqId) {
                return;
            }
            activeReqIdRef.current = reqId;
            setRunning(true);
        }).catch((err) => {
            console.error('[input] isRunning error', err);
        });
        return () => {
            disposed = true;
        };
    }, [aiChatItemService, chatId]);

    useEffect(() => {
        if (!chatId) {
            return;
        }
        return agentService.addEventListener(chatId, (arg: { context: RespSocketContext }) => {
            const item = arg.context.payload as AiChatItem;
            if (!item) {
                return;
            }
            handleAgentItem(item);
        });
    }, [agentService, chatId]);

    return {running, handleStop};
}
