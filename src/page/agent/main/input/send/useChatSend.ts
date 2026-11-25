import type {RefObject} from 'react';
import type {Editor} from '@tiptap/react';
import {useI18n} from '@/common/i18n/useI18n.ts';
import type {AiProvide} from '@/common/model/AiProvide.ts';
import {isEmpty, isNotEmpty} from '@/common/util/StringUtil.ts';
import type {AttachedImage} from '@/page/common/util/imageAttach.ts';
import {AI_CHAT_STATUS, AI_CHAT_TYPE, type AiChat} from '@/common/model/AiChat.ts';
import {useAgentService, useAiChatService} from '@/common/context/useAppHooks.tsx';
import {clearEditorContent, serializeEditorMessage} from '@/page/common/util/reference/editor.ts';
import type {AgentChatEvent, AgentChatImage, AgentQuestionAnswer, AgentQuestionAnswerEvent} from '@/common/model/req/event/WsAgent.ts';

export interface UseChatSendParams {
    running: boolean;
    sessionId: string;
    chatId: string | null;
    chatIdRef: RefObject<string | null>;
    confirmMode: number;
    images: AttachedImage[];
    selected: AiProvide | null;
    thinkingLevel: string | null;
    setChatId: (chatId: string | null) => void;
    setImages: (images: AttachedImage[]) => void;
}

export function useChatSend(params: UseChatSendParams) {
    const {t} = useI18n();
    const agentService = useAgentService();
    const aiChatService = useAiChatService();
    const {running, sessionId, chatId, chatIdRef, confirmMode, thinkingLevel, images, selected, setChatId, setImages} = params;

    function resolveChatImages(): AgentChatImage[] {
        return images.map(item => ({mediaType: item.mediaType, data: item.data}));
    }

    function resolveChatTitle(rawTitle: string): string {
        if (isNotEmpty(rawTitle)) {
            return rawTitle;
        }
        if (images.length > 0) {
            return t('agent.input.imageTitle');
        }
        return '';
    }

    function touchChat(aiChatId: string): void {
        aiChatService.touch(aiChatId).catch(err => {
            console.error('[input] touch aiChat error', err);
        });
    }

    function handleChat(aiChatId: string, target: Editor, submittedMessage?: string): void {
        if (!selected || isEmpty(selected.id)) {
            return;
        }
        const message = submittedMessage === undefined ? serializeEditorMessage(target).trim() : submittedMessage;
        const chatImages = resolveChatImages();
        if (isEmpty(message) && chatImages.length === 0) {
            return;
        }
        const event: AgentChatEvent = {
            data: {
                message: message,
                aiChatId: aiChatId,
                images: chatImages,
                confirmMode: confirmMode,
                aiProvideId: selected.id,
                thinkingLevel: thinkingLevel,
            },
        };
        agentService.handleChat(sessionId, event);
        if (!target.isDestroyed && (chatIdRef.current === aiChatId || (chatId === null && chatIdRef.current === null))) {
            clearEditorContent(target);
            setImages([]);
        }
        aiChatService.clearInputCache(aiChatId);
        touchChat(aiChatId);
    }

    function insertAiChat(target: Editor): void {
        const message = serializeEditorMessage(target).trim();
        const aiChat: AiChat = {id: String(), sort: -1, title: String(), timestamp: Date.now(), type: AI_CHAT_TYPE.CHAT, status: AI_CHAT_STATUS.NONE};
        aiChatService
            .insert(aiChat)
            .then(inserted => {
                aiChatService.clearInputCache(null);
                setChatId(inserted.id);
                handleChat(inserted.id, target, message);
            })
            .catch(err => {
                console.error('[input] insert aiChat error', err);
            });
    }

    function sendMessage(message: string): void {
        if (running || !selected || isEmpty(selected.id) || isEmpty(chatId)) {
            return;
        }
        const event: AgentChatEvent = {
            data: {
                images: [],
                message: message,
                aiChatId: chatId,
                confirmMode: confirmMode,
                aiProvideId: selected.id,
                thinkingLevel: thinkingLevel,
            },
        };
        agentService.handleChat(sessionId, event);
        touchChat(chatId);
    }

    function answerQuestion(answer: AgentQuestionAnswer): void {
        if (!selected || isEmpty(selected.id) || isEmpty(chatId)) {
            return;
        }
        const event: AgentQuestionAnswerEvent = {
            data: {
                ...answer,
                aiChatId: chatId,
                confirmMode: confirmMode,
                aiProvideId: selected.id,
                thinkingLevel: thinkingLevel,
            },
        };
        agentService.handleQuestionAnswer(sessionId, event);
        touchChat(chatId);
    }

    return {resolveChatTitle, handleChat, insertAiChat, sendMessage, answerQuestion};
}
