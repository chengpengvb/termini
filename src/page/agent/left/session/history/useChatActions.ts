import {useState, type MouseEvent} from 'react';
import {isEmpty} from '@/common/util/StringUtil.ts';
import type {AiChat} from '@/common/model/AiChat.ts';
import {useAiChatService} from '@/common/context/useAppHooks.tsx';

export interface ChatMenuContext {
    chat: AiChat;
    left: number;
    top: number;
}

export interface ChatRenameContext {
    chat: AiChat;
    name: string;
}

export interface UseChatActionsParams {
    chatId: string | null;
    setChatId: (chatId: string | null) => void;
    onChatDeleted: () => void;
}

export function useChatActions(params: UseChatActionsParams) {
    const aiChatService = useAiChatService();
    const {chatId, setChatId, onChatDeleted} = params;
    const [pendingDelete, setPendingDelete] = useState<AiChat | null>(null);
    const [menuContext, setMenuContext] = useState<ChatMenuContext | null>(null);
    const [pendingRename, setPendingRename] = useState<ChatRenameContext | null>(null);

    function openMenu(event: MouseEvent<HTMLElement>, chat: AiChat): void {
        setMenuContext({chat, left: event.clientX, top: event.clientY});
    }

    function closeMenu(): void {
        setMenuContext(null);
    }

    function handleRenameStart(): void {
        if (!menuContext) {
            return;
        }
        setPendingRename({chat: menuContext.chat, name: menuContext.chat.title});
    }

    function handleCancelRename(): void {
        setPendingRename(null);
    }

    function handleConfirmRename(): void {
        if (!pendingRename) {
            return;
        }
        const trimmed = pendingRename.name.trim();
        if (trimmed === '' || trimmed === pendingRename.chat.title) {
            setPendingRename(null);
            return;
        }
        const chatIdToRename = pendingRename.chat.id;
        setPendingRename(null);
        aiChatService.rename(chatIdToRename, trimmed).catch(error => {
            console.error('Failed to rename ai chat:', error);
        });
    }

    function handleDeleteFromMenu(): void {
        if (!menuContext) {
            return;
        }
        setPendingDelete(menuContext.chat);
    }

    function handleDeleteClick(event: MouseEvent<HTMLElement>, chat: AiChat): void {
        event.preventDefault();
        event.stopPropagation();
        if (document.activeElement instanceof HTMLElement) {
            document.activeElement.blur();
        }
        setPendingDelete(chat);
    }

    function handleCancelDelete(): void {
        setPendingDelete(null);
    }

    function handleConfirmDelete(): void {
        if (!pendingDelete || isEmpty(pendingDelete.id)) {
            return;
        }
        const deleteId = pendingDelete.id;
        setPendingDelete(null);
        onChatDeleted();
        aiChatService
            .deleteById(deleteId)
            .then(() => {
                if (chatId === deleteId) {
                    setChatId(null);
                }
            })
            .catch(error => {
                console.error('Failed to delete ai chat:', error);
            });
    }

    return {
        menuContext,
        pendingDelete,
        pendingRename,
        setPendingRename,
        openMenu,
        closeMenu,
        handleRenameStart,
        handleCancelRename,
        handleConfirmRename,
        handleDeleteFromMenu,
        handleDeleteClick,
        handleCancelDelete,
        handleConfirmDelete,
    };
}
