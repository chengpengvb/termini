import {useEffect, useRef} from 'react';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {isNotNil} from '@/common/util/StringUtil.ts';
import type {AiChatItem} from '@/common/model/AiChatItem.ts';
import {isChatOriginEcho} from '@/common/const/ChatOriginConst.ts';
import {useSystemUtilService} from '@/common/context/useAppHooks.tsx';
import {RESP_EVENT_AGENT} from '@/common/model/resp/event/RespEventAgent.ts';

const NOTIFY_BODY_MAX = 120;

export interface UseRunOutcomeParams {
    onInterruptedWithoutOutput?: (message: string) => void;
}

export function useRunOutcome(params: UseRunOutcomeParams) {
    const {t} = useI18n();
    const attentionRef = useRef(false);
    const runOutputRef = useRef(false);
    const runFailedRef = useRef(false);
    const {onInterruptedWithoutOutput} = params;
    const systemUtilService = useSystemUtilService();
    const runUserMessageRef = useRef<string | null>(null);
    const lastAssistantTextRef = useRef<string | null>(null);

    function requestAttention(): void {
        if (document.hasFocus()) {
            return;
        }
        attentionRef.current = true;
        systemUtilService.requestUserAttention(true).catch(err => {
            console.error('[agent] request user attention error', err);
        });
    }

    function resetRun(): void {
        runOutputRef.current = false;
        runFailedRef.current = false;
        lastAssistantTextRef.current = null;
    }

    function trackRunOutput(item: AiChatItem): void {
        if (item.type === RESP_EVENT_AGENT.ON_USER) {
            runUserMessageRef.current = isChatOriginEcho(item.context.origin) ? item.context.message ?? null : null;
            return;
        }
        if (item.type === RESP_EVENT_AGENT.ON_ERROR) {
            runFailedRef.current = true;
            return;
        }
        runOutputRef.current = true;
        if (item.type === RESP_EVENT_AGENT.ON_TEXT_FINAL && item.context.delta) {
            lastAssistantTextRef.current = item.context.delta;
        }
    }

    function finishRun(endType: number): void {
        if (endType === RESP_EVENT_AGENT.ON_CANCELLED) {
            const message = runUserMessageRef.current;
            if (!runOutputRef.current && isNotNil(message) && isNotNil(onInterruptedWithoutOutput)) {
                onInterruptedWithoutOutput(message);
            }
            return;
        }
        if (document.hasFocus()) {
            return;
        }
        const title = runFailedRef.current ? t('agent.notify.failedTitle') : t('agent.notify.doneTitle');
        const text = (lastAssistantTextRef.current ?? '').replace(/\s+/g, ' ').trim();
        const body = text.length > 0 ? (text.length > NOTIFY_BODY_MAX ? `${text.slice(0, NOTIFY_BODY_MAX)}…` : text) : t('agent.notify.emptyBody');
        systemUtilService.notify(title, body).catch(err => {
            console.error('[agent] notify error', err);
        });
    }

    useEffect(() => {
        function handleWindowFocus(): void {
            if (!attentionRef.current) {
                return;
            }
            attentionRef.current = false;
            systemUtilService.requestUserAttention(false).catch(err => {
                console.error('[agent] clear user attention error', err);
            });
        }

        window.addEventListener('focus', handleWindowFocus);
        return () => {
            window.removeEventListener('focus', handleWindowFocus);
        };
    }, [systemUtilService]);

    return {requestAttention, resetRun, trackRunOutput, finishRun};
}
