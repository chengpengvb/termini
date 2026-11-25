import {useEffect, useMemo, useState} from 'react';
import {isNotNil} from '@/common/util/StringUtil.ts';
import type {AiChat} from '@/common/model/AiChat.ts';
import type {AiProject} from '@/common/model/AiProject.ts';
import {useAiChat2ProjectService, useAiChatService, useAiProjectService} from '@/common/context/useAppHooks.tsx';

export function useChatHistoryData() {
    const aiChatService = useAiChatService();
    const aiProjectService = useAiProjectService();
    const aiChat2ProjectService = useAiChat2ProjectService();
    const [chatList, setChatList] = useState<AiChat[]>([]);
    const [projectList, setProjectList] = useState<AiProject[]>([]);
    const [projectMap, setProjectMap] = useState<Map<string, string>>(new Map());

    function currentProjectId(chat: AiChat): string | null {
        return projectMap.get(chat.id) ?? null;
    }

    useEffect(() => {
        aiChatService
            .selectAll()
            .then(setChatList)
            .catch(error => {
                console.error('Failed to load ai chat list:', error);
            });
        aiProjectService
            .selectAll()
            .then(setProjectList)
            .catch(error => {
                console.error('Failed to load ai project list:', error);
            });
        aiChat2ProjectService
            .list()
            .then(links => {
                setProjectMap(new Map(links.map(item => [item.chat_id, item.project_id])));
            })
            .catch(error => {
                console.error('Failed to load ai chat 2 project list:', error);
            });
        const unsubscribeChat = aiChatService.addAiChatListChangeListener(setChatList);
        const unsubscribeProject = aiProjectService.addAiProjectListChangeListener(setProjectList);
        const unsubscribeLink = aiChat2ProjectService.addAiChat2ProjectListChangeListener(links => {
            setProjectMap(new Map(links.map(item => [item.chat_id, item.project_id])));
        });
        return () => {
            unsubscribeChat();
            unsubscribeProject();
            unsubscribeLink();
        };
    }, [aiChatService, aiProjectService, aiChat2ProjectService]);

    return {chatList, projectList, projectMap, currentProjectId};
}

export function useChatBuckets(visibleChats: AiChat[], projectMap: Map<string, string>) {
    return useMemo(() => {
        const byProject = new Map<string, AiChat[]>();
        const unlinked: AiChat[] = [];
        for (const chat of visibleChats) {
            const projectId = projectMap.get(chat.id);
            if (isNotNil(projectId)) {
                const bucket = byProject.get(projectId);
                if (bucket) {
                    bucket.push(chat);
                } else {
                    byProject.set(projectId, [chat]);
                }
                continue;
            }
            unlinked.push(chat);
        }
        return {chatsByProject: byProject, unlinkedChats: unlinked};
    }, [visibleChats, projectMap]);
}
