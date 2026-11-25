import {Box, ButtonBase} from '@mui/material';
import {useState, type ReactNode} from 'react';
import {useI18n} from '@/common/i18n/useI18n.ts';
import type {QueuedChatMessage} from '@/common/model/dto/QueuedChatMessage.ts';
import {DeleteOutline, EditOutlined, KeyboardArrowDown, KeyboardArrowRight, KeyboardArrowUp, KeyboardReturn} from '@mui/icons-material';

interface QueuedMessagesProps {
    queue: QueuedChatMessage[];
    editingId: string | null;
    onEdit: (item: QueuedChatMessage) => void;
    onDelete: (item: QueuedChatMessage) => void;
    onSendNow: (item: QueuedChatMessage) => void;
    onMove: (item: QueuedChatMessage, delta: number) => void;
}

export function QueuedMessages(props: QueuedMessagesProps) {
    const {t, tf} = useI18n();
    const {queue, editingId, onEdit, onDelete, onSendNow, onMove} = props;
    const [expanded, setExpanded] = useState<boolean>(true);

    function resolveHeader(): string {
        if (queue.length === 1) {
            return t('agent.queue.titleOne');
        }
        return tf('agent.queue.titleMany', {count: queue.length});
    }

    function resolveRowText(item: QueuedChatMessage): string {
        const firstLine = item.message.split('\n')[0].trim();
        if (firstLine.length > 0) {
            return firstLine;
        }
        return tf('agent.queue.imagesBadge', {count: item.images.length});
    }

    function resolveRowClassName(item: QueuedChatMessage): string {
        if (item.id === editingId) {
            return 'tm-agent-queue-row is-editing';
        }
        return 'tm-agent-queue-row';
    }

    function handleToggle(): void {
        setExpanded(prev => !prev);
    }

    function renderMove(item: QueuedChatMessage, index: number): ReactNode {
        if (queue.length < 2) {
            return null;
        }
        return (
            <Box className="tm-agent-queue-move-group">
                <ButtonBase disableRipple className="tm-agent-queue-move" disabled={index === 0} title={t('agent.queue.moveUp')} aria-label={t('agent.queue.moveUp')} onClick={() => onMove(item, -1)}>
                    <KeyboardArrowUp sx={{fontSize: 15}}/>
                </ButtonBase>
                <ButtonBase disableRipple className="tm-agent-queue-move" disabled={index === queue.length - 1} title={t('agent.queue.moveDown')} aria-label={t('agent.queue.moveDown')} onClick={() => onMove(item, 1)}>
                    <KeyboardArrowDown sx={{fontSize: 15}}/>
                </ButtonBase>
            </Box>
        );
    }

    if (queue.length === 0) {
        return null;
    }

    return (
        <Box className="tm-agent-daemon">
            <ButtonBase disableRipple className="tm-agent-daemon-head" onClick={handleToggle}>
                {expanded ? <KeyboardArrowDown className="tm-agent-daemon-chevron"/> : <KeyboardArrowRight className="tm-agent-daemon-chevron"/>}
                <Box className="tm-agent-daemon-title">{resolveHeader()}</Box>
            </ButtonBase>
            {expanded ? (
                <Box className="tm-agent-queue-list">
                    {queue.map((item, index) => (
                        <Box key={item.id} className={resolveRowClassName(item)}>
                            <Box className="tm-agent-queue-icon"/>
                            <Box className="tm-agent-queue-text" title={item.message}>{resolveRowText(item)}</Box>
                            <Box className="tm-agent-queue-actions">
                                {renderMove(item, index)}
                                <ButtonBase disableRipple className="tm-agent-queue-send-now" title={t('agent.queue.sendNow')} aria-label={t('agent.queue.sendNow')} onClick={() => onSendNow(item)}>
                                    <KeyboardReturn sx={{fontSize: 14}}/>
                                </ButtonBase>
                                <ButtonBase disableRipple className="tm-agent-queue-action" aria-label={t('agent.queue.edit')} onClick={() => onEdit(item)}>
                                    <EditOutlined sx={{fontSize: 14}}/>
                                </ButtonBase>
                                <ButtonBase disableRipple className="tm-agent-queue-action" aria-label={t('agent.queue.delete')} onClick={() => onDelete(item)}>
                                    <DeleteOutline sx={{fontSize: 14}}/>
                                </ButtonBase>
                            </Box>
                        </Box>
                    ))}
                </Box>
            ) : null}
        </Box>
    );
}
