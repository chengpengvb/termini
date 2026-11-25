import {Box} from '@mui/material';
import {Ref, useState} from 'react';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {isNil, isNotNil} from '@/common/util/StringUtil.ts';
import type {AiChatItem} from '@/common/model/AiChatItem.ts';
import {RefMsg} from '@/page/common/util/reference/RefMsg.tsx';
import {isChatOriginEcho} from '@/common/const/ChatOriginConst.ts';
import {useItemHandle} from '@/page/common/hooks/useItemHandle.ts';
import type {ItemHandle} from '@/page/common/dto/agent/ItemHandle.ts';
import {formatFullDateTime, formatMessageTime} from '@/page/common/util/agentTimeFormat.ts';

interface ItemUserProps {
    item: AiChatItem;
    canEdit?: boolean;
    editing?: boolean;
    ref?: Ref<ItemHandle>;
    onRevertEdit?: (itemId: string, message: string) => void;
}

export function ItemUser(props: ItemUserProps) {
    const {tf} = useI18n();
    const {item, ref, canEdit, editing, onRevertEdit} = props;
    const [content, setContent] = useState<string | null>(null);

    function onEvent(eventItem: AiChatItem): void {
        if (!isChatOriginEcho(eventItem.context.origin)) {
            return;
        }
        const message = eventItem.context.message;
        if (!message) {
            return;
        }
        setContent(message);
    }

    function canRevertEdit(): boolean {
        if (canEdit !== true) {
            return false;
        }
        if (editing === true) {
            return false;
        }
        return isNotNil(onRevertEdit);
    }

    function handleRevertEdit(): void {
        if (isNil(content)) {
            return;
        }
        if (!onRevertEdit) {
            return;
        }
        onRevertEdit(item.id, content);
    }

    function renderSentAt() {
        if (editing === true || item.timestamp <= 0) {
            return null;
        }
        return (
            <Box component="time" className="tm-agent-msg-user-meta" title={tf('agent.time.sentAt', {time: formatFullDateTime(item.timestamp)})}>
                {formatMessageTime(item.timestamp)}
            </Box>
        );
    }

    useItemHandle(ref, onEvent);

    if (isNil(content)) {
        return null;
    }

    return (
        <Box className="tm-agent-msg tm-agent-msg-user" data-agent-user-item={item.id}>
            <Box className={editing === true ? 'tm-agent-msg-user-bubble is-editing-ghost' : canRevertEdit() ? 'tm-agent-msg-user-bubble is-editable' : 'tm-agent-msg-user-bubble'} data-tm-user-bubble="true" data-tm-user-editable={canRevertEdit() ? 'true' : undefined} onClick={canRevertEdit() ? handleRevertEdit : undefined}>
                <RefMsg text={content} foldChatText={true}/>
            </Box>
            {renderSentAt()}
        </Box>
    );
}
