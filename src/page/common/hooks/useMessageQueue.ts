import type {Editor} from '@tiptap/react';
import {generateId} from '@/common/util/IdUtil.ts';
import type {AiProvide} from '@/common/model/AiProvide.ts';
import {RefObject, useEffect, useRef, useState} from 'react';
import type {AttachedImage} from '@/page/common/util/imageAttach.ts';
import type {AgentChatEvent} from '@/common/model/req/event/WsAgent.ts';
import type {QueuedChatMessage} from '@/common/model/dto/QueuedChatMessage.ts';
import {isEmpty, isNil, isNotEmpty, isNotNil} from '@/common/util/StringUtil.ts';
import {useAgentService, useAiChatService} from '@/common/context/useAppHooks.tsx';
import {clearEditorContent, focusEditor, serializeEditorMessage, setEditorMessage} from '@/page/common/util/reference/editor.ts';

export interface UseMessageQueueParams {
    running: boolean;
    sessionId: string;
    onStop: () => void;
    chatId: string | null;
    images: AttachedImage[];
    onCancelRevert: () => void;
    revertItemId: string | null;
    confirmModeRef: RefObject<number>;
    isRevertingRef: RefObject<boolean>;
    resolveEditor: () => Editor | null;
    chatIdRef: RefObject<string | null>;
    setHasAny: (hasAny: boolean) => void;
    selectedRef: RefObject<AiProvide | null>;
    thinkingLevelRef: RefObject<string | null>;
    setImages: (images: AttachedImage[]) => void;
}

export function useMessageQueue(params: UseMessageQueueParams) {
    const agentService = useAgentService();
    const aiChatService = useAiChatService();
    const queueRef = useRef<QueuedChatMessage[]>([]);
    const forceSendIdRef = useRef<string | null>(null);
    const editingQueueIdRef = useRef<string | null>(null);
    const preQueueEditImagesRef = useRef<AttachedImage[]>([]);
    const [queue, setQueue] = useState<QueuedChatMessage[]>([]);
    const [editingQueueId, setEditingQueueId] = useState<string | null>(null);
    const preQueueEditContentRef = useRef<ReturnType<Editor['getJSON']> | null>(null);
    const {chatId, sessionId, running, images, chatIdRef, selectedRef, confirmModeRef, thinkingLevelRef, isRevertingRef, revertItemId, resolveEditor, setImages, setHasAny, onStop, onCancelRevert} = params;

    queueRef.current = queue;
    editingQueueIdRef.current = editingQueueId;

    function setQueueAndCache(next: QueuedChatMessage[]): void {
        queueRef.current = next;
        setQueue(next);
        const currentChatId = chatIdRef.current;
        if (isNotEmpty(currentChatId)) {
            aiChatService.setMessageQueue(currentChatId, next);
        }
    }

    function sendChatRaw(item: QueuedChatMessage): boolean {
        const currentChatId = chatIdRef.current;
        const provide = selectedRef.current;
        if (isEmpty(currentChatId) || !provide || isEmpty(provide.id)) {
            return false;
        }
        const event: AgentChatEvent = {
            data: {
                message: item.message,
                aiChatId: currentChatId,
                images: item.images.map(image => ({mediaType: image.mediaType, data: image.data})),
                confirmMode: confirmModeRef.current,
                aiProvideId: provide.id,
                thinkingLevel: thinkingLevelRef.current,
            },
        };
        agentService.handleChat(sessionId, event);
        aiChatService.touch(currentChatId).catch((err) => {
            console.error('[input] touch aiChat error', err);
        });
        return true;
    }

    function enqueueFromEditor(target: Editor): void {
        const message = serializeEditorMessage(target).trim();
        if (isEmpty(message) && images.length === 0) {
            return;
        }
        const queued: QueuedChatMessage = {id: generateId(), message, images};
        setQueueAndCache([...queueRef.current, queued]);
        clearEditorContent(target);
        setImages([]);
        aiChatService.clearInputCache(chatIdRef.current);
        setHasAny(false);
    }

    function advanceQueue(): void {
        if (editingQueueIdRef.current !== null || isRevertingRef.current) {
            return;
        }
        const list = queueRef.current;
        if (list.length === 0) {
            return;
        }
        if (!sendChatRaw(list[0])) {
            return;
        }
        setQueueAndCache(list.slice(1));
    }

    function consumeForceSend(): boolean {
        const id = forceSendIdRef.current;
        if (id === null) {
            return false;
        }
        forceSendIdRef.current = null;
        const list = queueRef.current;
        const item = list.find(row => row.id === id);
        if (!item) {
            return false;
        }
        if (!sendChatRaw(item)) {
            return false;
        }
        setQueueAndCache(list.filter(row => row.id !== id));
        return true;
    }

    function forceSendQueued(item: QueuedChatMessage): void {
        if (editingQueueIdRef.current === item.id) {
            cancelQueueEdit();
        }
        if (!running) {
            if (!sendChatRaw(item)) {
                return;
            }
            setQueueAndCache(queueRef.current.filter(row => row.id !== item.id));
            return;
        }
        forceSendIdRef.current = item.id;
        onStop();
    }

    function removeQueued(item: QueuedChatMessage): void {
        if (editingQueueIdRef.current === item.id) {
            cancelQueueEdit();
        }
        setQueueAndCache(queueRef.current.filter(row => row.id !== item.id));
    }

    function moveQueued(item: QueuedChatMessage, delta: number): void {
        const list = queueRef.current;
        const index = list.findIndex(row => row.id === item.id);
        if (index < 0) {
            return;
        }
        const target = index + delta;
        if (target < 0 || target >= list.length) {
            return;
        }
        const next = [...list];
        next[index] = list[target];
        next[target] = list[index];
        setQueueAndCache(next);
    }

    function beginQueueEdit(item: QueuedChatMessage): void {
        const target = resolveEditor();
        if (!target) {
            return;
        }
        if (isNotNil(revertItemId)) {
            onCancelRevert();
        }
        const editingId = editingQueueIdRef.current;
        if (editingId === null) {
            preQueueEditContentRef.current = target.isEmpty ? null : target.getJSON();
            preQueueEditImagesRef.current = images;
        } else {
            const message = serializeEditorMessage(target).trim();
            setQueueAndCache(queueRef.current.map(row => row.id === editingId ? {...row, message, images} : row));
        }
        editingQueueIdRef.current = item.id;
        setEditingQueueId(item.id);
        setEditorMessage(target, item.message);
        setImages(item.images);
        setHasAny(!target.isEmpty);
        focusEditor(target);
    }

    function cancelQueueEdit(): void {
        if (editingQueueIdRef.current === null) {
            return;
        }
        editingQueueIdRef.current = null;
        setEditingQueueId(null);
        const target = resolveEditor();
        if (!target) {
            return;
        }
        const prev = preQueueEditContentRef.current;
        preQueueEditContentRef.current = null;
        if (prev === null) {
            clearEditorContent(target);
            aiChatService.clearInputCache(chatIdRef.current);
        } else {
            target.commands.setContent(prev);
            aiChatService.setInputCache(chatIdRef.current, {content: prev});
        }
        setImages(preQueueEditImagesRef.current);
        preQueueEditImagesRef.current = [];
        setHasAny(!target.isEmpty);
    }

    function submitQueueEdit(target: Editor): void {
        const id = editingQueueIdRef.current;
        if (id === null) {
            return;
        }
        const message = serializeEditorMessage(target).trim();
        if (isEmpty(message) && images.length === 0) {
            return;
        }
        const edited: QueuedChatMessage = {id, message, images};
        if (running) {
            setQueueAndCache(queueRef.current.map(row => row.id === id ? edited : row));
            cancelQueueEdit();
            return;
        }
        setQueueAndCache(queueRef.current.filter(row => row.id !== id));
        cancelQueueEdit();
        sendChatRaw(edited);
    }

    useEffect(() => {
        editingQueueIdRef.current = null;
        setEditingQueueId(null);
        forceSendIdRef.current = null;
        preQueueEditContentRef.current = null;
        preQueueEditImagesRef.current = [];
        const nextQueue = isNotEmpty(chatId) ? aiChatService.getMessageQueue(chatId) : [];
        queueRef.current = nextQueue;
        setQueue(nextQueue);
    }, [aiChatService, chatId]);

    useEffect(() => {
        if (isNil(editingQueueId)) {
            return;
        }
        function onKeyDown(event: KeyboardEvent): void {
            if (event.key !== 'Escape') {
                return;
            }
            event.preventDefault();
            cancelQueueEdit();
        }
        document.addEventListener('keydown', onKeyDown, true);
        return () => {
            document.removeEventListener('keydown', onKeyDown, true);
        };
    }, [editingQueueId]);

    return {queue, editingQueueId, enqueueFromEditor, advanceQueue, consumeForceSend, forceSendQueued, removeQueued, moveQueued, beginQueueEdit, cancelQueueEdit, submitQueueEdit};
}
