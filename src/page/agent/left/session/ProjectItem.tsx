import type {MouseEvent} from 'react';
import {useI18n} from '@/common/i18n/useI18n.ts';
import type {AiChat} from '@/common/model/AiChat.ts';
import {ChevronRight, Folder} from '@mui/icons-material';
import type {AiProject} from '@/common/model/AiProject.ts';
import {Box, ButtonBase, Collapse, Typography} from '@mui/material';
import {HistoryItem} from '@/page/agent/left/session/HistoryItem.tsx';
import {useLongPressMenu} from '@/page/common/hooks/useLongPressMenu.ts';

export interface ProjectItemProps {
    chats: AiChat[];
    project: AiProject;
    expanded: boolean;
    mobileSwipe: boolean;
    activeChatId: string | null;
    swipeOpenChatId: string | null;
    onToggle: (id: string) => void;
    onSelectChat: (id: string) => void;
    onChatSwipeOpen: (id: string | null) => void;
    onChatDelete: (e: MouseEvent<HTMLElement>, chat: AiChat) => void;
    onChatContextMenu: (e: MouseEvent<HTMLElement>, chat: AiChat) => void;
    onProjectContextMenu: (e: MouseEvent<HTMLElement>, project: AiProject) => void;
}

export function ProjectItem(props: ProjectItemProps) {
    const {t} = useI18n();
    const {project, chats, expanded, activeChatId, swipeOpenChatId, mobileSwipe, onToggle, onSelectChat, onChatSwipeOpen, onChatDelete, onChatContextMenu, onProjectContextMenu} = props;

    const longPress = useLongPressMenu((clientX, clientY) => {
        onProjectContextMenu({preventDefault: () => undefined, stopPropagation: () => undefined, clientX, clientY} as MouseEvent<HTMLElement>, project);
    });

    const handleToggle = (): void => {
        if (longPress.consumeIfFired()) {
            return;
        }
        onToggle(project.id);
    };


    return (
        <Box className={expanded ? 'tm-agent-project-item is-expanded' : 'tm-agent-project-item'}>
            <ButtonBase disableRipple className="tm-agent-nav-item" onClick={handleToggle} onContextMenu={longPress.handleContextMenu} title={project.name} onPointerDown={longPress.handlePointerDown} onPointerMove={longPress.handlePointerMove} onPointerUp={longPress.handlePointerEnd} onPointerCancel={longPress.handlePointerEnd} onPointerLeave={longPress.handlePointerEnd}>
                <Box className="tm-agent-nav-item-icon-slot" aria-hidden>
                    <Folder className="tm-agent-nav-item-icon"/>
                </Box>
                <Box className="tm-agent-history-label">
                    <Typography component="span" className="tm-agent-nav-item-label">{project.name}</Typography>
                </Box>
                <ChevronRight className={expanded ? 'tm-agent-nav-item-chevron is-expanded' : 'tm-agent-nav-item-chevron'}/>
            </ButtonBase>
            <Collapse in={expanded} timeout={220} sx={{transitionTimingFunction: 'var(--tm-easing-ios)'}}>
                <Box className="tm-agent-project-children">
                    {chats.length === 0 ? (
                        <Typography className="tm-agent-left-placeholder">{t('agent.left.noProjectChats')}</Typography>
                    ) : (
                        chats.map((chat) => (
                            <HistoryItem
                                key={chat.id}
                                chat={chat}
                                selected={activeChatId === chat.id}
                                mobileSwipe={mobileSwipe}
                                swipeOpen={swipeOpenChatId === chat.id}
                                onSelect={onSelectChat}
                                onSwipeOpen={onChatSwipeOpen}
                                onDeleteClick={onChatDelete}
                                onContextMenu={onChatContextMenu}
                            />
                        ))
                    )}
                </Box>
            </Collapse>
        </Box>
    );
}
