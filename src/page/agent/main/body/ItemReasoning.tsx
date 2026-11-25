import {Box} from '@mui/material';
import {Ref, useState} from 'react';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {isNil} from '@/common/util/StringUtil.ts';
import type {AiChatItem} from '@/common/model/AiChatItem.ts';
import {StreamFold} from '@/page/agent/common/StreamFold.tsx';
import {useItemHandle} from '@/page/common/hooks/useItemHandle.ts';
import {AgentMarkdown} from '@/page/agent/common/AgentMarkdown.tsx';
import type {ItemHandle} from '@/page/common/dto/agent/ItemHandle.ts';
import {RESP_EVENT_AGENT} from '@/common/model/resp/event/RespEventAgent.ts';

interface ItemReasoningProps {
    item: AiChatItem;
    ref?: Ref<ItemHandle>;
}

export function ItemReasoning(props: ItemReasoningProps) {
    const {ref} = props;
    const {t} = useI18n();
    const [done, setDone] = useState(false);
    const [content, setContent] = useState<string | null>(null);

    function onEvent(eventItem: AiChatItem): void {
        if (eventItem.type === RESP_EVENT_AGENT.ON_REASONING) {
            setDone(false);
            setContent((prev: string | null) => {
                if (isNil(prev)) {
                    return eventItem.context.delta;
                }
                return prev + eventItem.context.delta;
            });
            return;
        }
        if (eventItem.type === RESP_EVENT_AGENT.ON_REASONING_FINAL) {
            setDone(true);
            setContent(eventItem.context.delta);
            return;
        }
        if (eventItem.type === RESP_EVENT_AGENT.ON_DONE || eventItem.type === RESP_EVENT_AGENT.ON_CANCELLED) {
            setDone(true);
        }
    }

    useItemHandle(ref, onEvent);

    if (isNil(content)) {
        return null;
    }

    return (
        <Box className="tm-agent-msg tm-agent-msg-assistant">
            <StreamFold done={done} scrollKey={content} streamingLabel={t('agent.reasoning.streaming')} doneLabel={t('agent.reasoning.done')}>
                <AgentMarkdown content={content} className="tm-agent-md tm-agent-md-reasoning"/>
            </StreamFold>
        </Box>
    );
}
