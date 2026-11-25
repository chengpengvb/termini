import {useEffect, useState} from 'react';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {isNotNil} from '@/common/util/StringUtil.ts';
import type {AiChat} from '@/common/model/AiChat.ts';
import {exportChatMarkdown} from '@/page/common/util/chatExport.ts';
import {useAiChatItemService, useFileEditService} from '@/common/context/useAppHooks.tsx';

const NOTICE_DISMISS_MS = 4000;

export function useChatExport() {
    const {tf} = useI18n();
    const fileEditService = useFileEditService();
    const aiChatItemService = useAiChatItemService();
    const [exportNotice, setExportNotice] = useState<string | null>(null);

    function exportChat(chat: AiChat): void {
        exportChatMarkdown(chat, aiChatItemService, fileEditService)
            .then(path => {
                if (isNotNil(path)) {
                    setExportNotice(tf('agent.export.done', {path}));
                }
            })
            .catch(error => {
                console.error('Failed to export ai chat:', error);
                setExportNotice(tf('agent.export.failed', {error: String(error)}));
            });
    }

    useEffect(() => {
        if (exportNotice === null) {
            return;
        }
        const timer = window.setTimeout(() => setExportNotice(null), NOTICE_DISMISS_MS);
        return () => {
            window.clearTimeout(timer);
        };
    }, [exportNotice]);

    return {exportNotice, exportChat};
}
