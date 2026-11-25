import {useEffect, useState} from 'react';
import type {AiChatItem} from '@/common/model/AiChatItem.ts';
import type {RespSocketContext} from '@/common/model/resp/RespWsContext.ts';
import {RESP_EVENT_AGENT} from '@/common/model/resp/event/RespEventAgent.ts';
import {useAgentService, useAiAgentApiService} from '@/common/context/useAppHooks.tsx';
import type {RespAgentShellDaemon, RespAgentShellDaemonRow} from '@/common/model/resp/RespAgent.ts';

export function useShellDaemons(chatId: string | null): RespAgentShellDaemonRow[] {
    const agentService = useAgentService();
    const aiAgentApiService = useAiAgentApiService();
    const [daemons, setDaemons] = useState<RespAgentShellDaemonRow[]>([]);

    useEffect(() => {
        setDaemons([]);
        if (!chatId) {
            return;
        }
        let disposed = false;
        aiAgentApiService.listShellDaemons(chatId).then((rows) => {
            if (!disposed) {
                setDaemons(rows);
            }
        }).catch((err) => {
            console.error('[daemon] list error', err);
        });
        const off = agentService.addEventListener(chatId, (arg: {context: RespSocketContext}) => {
            const item = arg.context.payload as AiChatItem;
            if (!item || item.type !== RESP_EVENT_AGENT.ON_SHELL_DAEMON) {
                return;
            }
            const payload = item.context as RespAgentShellDaemon;
            if (payload.chatId !== chatId) {
                return;
            }
            setDaemons(payload.daemons);
        });
        return () => {
            disposed = true;
            off();
        };
    }, [agentService, aiAgentApiService, chatId]);

    return daemons;
}
