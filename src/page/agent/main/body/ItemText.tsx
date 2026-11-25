import {Box} from '@mui/material';
import {Ref, useState} from 'react';
import {isNil} from '@/common/util/StringUtil.ts';
import type {AiChatItem} from '@/common/model/AiChatItem.ts';
import {useItemHandle} from '@/page/common/hooks/useItemHandle.ts';
import {AgentMarkdown} from '@/page/agent/common/AgentMarkdown.tsx';
import type {ItemHandle} from '@/page/common/dto/agent/ItemHandle.ts';
import {RESP_EVENT_AGENT} from '@/common/model/resp/event/RespEventAgent.ts';

interface ItemTextProps {
    item: AiChatItem;
    ref?: Ref<ItemHandle>;
}

export function ItemText(props: ItemTextProps) {
    const {ref} = props;
    const [content, setContent] = useState<string | null>(null);

    function onEvent(eventItem: AiChatItem): void {
        if (eventItem.type === RESP_EVENT_AGENT.ON_TEXT) {
            setContent((prev: string | null) => {
                if (isNil(prev)) {
                    return eventItem.context.delta;
                }
                return prev + eventItem.context.delta;
            });
            return;
        }
        if (eventItem.type === RESP_EVENT_AGENT.ON_TEXT_FINAL) {
            setContent(eventItem.context.delta);
        }
    }

    useItemHandle(ref, onEvent);

    if (isNil(content)) {
        return null;
    }

    return (
        <Box className="tm-agent-msg tm-agent-msg-assistant">
            <AgentMarkdown content={content} className="tm-agent-md"/>
        </Box>
    );
}
