import type {AiChat} from '@/common/model/AiChat.ts';
import {useEffect, useMemo, useRef, useState} from 'react';
import {useAiChatItemService} from '@/common/context/useAppHooks.tsx';

const SEARCH_DEBOUNCE_MS = 300;

export function useChatSearch(chatList: AiChat[]) {
    const searchTicketRef = useRef(0);
    const [keyword, setKeyword] = useState('');
    const aiChatItemService = useAiChatItemService();
    const [matchedIds, setMatchedIds] = useState<ReadonlySet<string> | null>(null);
    const searching = keyword.trim().length > 0;

    const visibleChats = useMemo(() => {
        if (!searching) {
            return chatList;
        }
        const lower = keyword.trim().toLowerCase();
        return chatList.filter(chat => chat.title.toLowerCase().includes(lower) || (matchedIds?.has(chat.id) ?? false));
    }, [chatList, keyword, matchedIds, searching]);

    useEffect(() => {
        const trimmed = keyword.trim();
        if (trimmed.length === 0) {
            setMatchedIds(null);
            return;
        }
        const ticket = ++searchTicketRef.current;
        const timer = window.setTimeout(() => {
            aiChatItemService
                .searchChatIds(trimmed)
                .then(ids => {
                    if (ticket === searchTicketRef.current) {
                        setMatchedIds(new Set(ids));
                    }
                })
                .catch(error => {
                    console.error('Failed to search ai chat items:', error);
                });
        }, SEARCH_DEBOUNCE_MS);
        return () => {
            window.clearTimeout(timer);
        };
    }, [aiChatItemService, keyword]);

    return {keyword, setKeyword, searching, visibleChats};
}
