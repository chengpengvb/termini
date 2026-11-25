import type {Editor} from '@tiptap/react';
import type {JSONContent} from '@tiptap/core';
import {isEmpty, isNil} from '@/common/util/StringUtil.ts';
import {useAiChatService} from '@/common/context/useAppHooks.tsx';
import {resolveErrorText} from '@/common/model/resp/ResultModel.ts';
import {usePlatformMobile} from '@/common/context/usePlatformMobile';
import {RefObject, useEffect, useLayoutEffect, useRef, useState} from 'react';
import {clearEditorContent, focusEditor, serializeEditorMessage, setEditorMessage} from '@/page/common/util/reference/editor.ts';
import {ensureInlineRevertInView, resolveInlineRevertBox, resolveRevertBubble, type InlineRevertBox} from '@/page/common/util/inlineRevertLayout.ts';

export interface UseInlineRevertParams {
    chatId: string | null;
    onReverted: () => void;
    onBeginRevert: () => void;
    setHasAny: (hasAny: boolean) => void;
    resolveEditor: () => Editor | null;
    chatIdRef: RefObject<string | null>;
    wrapRef: RefObject<HTMLDivElement | null>;
    onRevertChange?: (itemId: string | null) => void;
    sendChat: (aiChatId: string, target: Editor, submittedMessage?: string) => void;
}

export function useInlineRevert(params: UseInlineRevertParams) {
    const mobile = usePlatformMobile();
    const aiChatService = useAiChatService();
    const isRevertingRef = useRef<boolean>(false);
    const submittingRef = useRef(false);
    const followupRef = useRef<HTMLDivElement | null>(null);
    const [revertAsking, setRevertAsking] = useState<boolean>(false);
    const [revertSubmitting, setRevertSubmitting] = useState(false);
    const [revertError, setRevertError] = useState<string | null>(null);
    const [revertItemId, setRevertItemId] = useState<string | null>(null);
    const [followupHeight, setFollowupHeight] = useState<number | null>(null);
    const preRevertContentRef = useRef<JSONContent | null>(null);
    const [inlineRevertBox, setInlineRevertBox] = useState<InlineRevertBox | null>(null);
    const {chatId, chatIdRef, wrapRef, resolveEditor, setHasAny, onReverted, onRevertChange, onBeginRevert, sendChat} = params;

    function beginRevert(itemId: string, message: string): void {
        const target = resolveEditor();
        if (!target) {
            return;
        }
        if (isRevertingRef.current || submittingRef.current) {
            focusEditor(target);
            return;
        }
        onBeginRevert();
        preRevertContentRef.current = target.isEmpty ? null : target.getJSON();
        aiChatService.setInputCache(chatId, {content: preRevertContentRef.current, revert: {itemId, originalContent: preRevertContentRef.current}});
        isRevertingRef.current = true;
        setRevertItemId(itemId);
        setRevertError(null);
        setEditorMessage(target, message);
        setHasAny(!target.isEmpty);
    }

    function clearInlineRevertLayout(): void {
        setFollowupHeight(null);
        setInlineRevertBox(null);
    }

    function cancelRevert(): void {
        if (submittingRef.current) {
            return;
        }
        setRevertItemId(null);
        setRevertAsking(false);
        setRevertError(null);
        clearInlineRevertLayout();
        if (!isRevertingRef.current) {
            return;
        }
        isRevertingRef.current = false;
        const target = resolveEditor();
        if (!target) {
            return;
        }
        const prev = preRevertContentRef.current;
        preRevertContentRef.current = null;
        aiChatService.setInputCache(chatIdRef.current, {content: prev});
        if (prev === null) {
            clearEditorContent(target);
            aiChatService.clearInputCache(chatIdRef.current);
        } else {
            target.commands.setContent(prev);
        }
        setHasAny(!target.isEmpty);
    }

    function askRevert(): void {
        if (submittingRef.current) {
            return;
        }
        setRevertError(null);
        setRevertAsking(true);
    }

    function dismissRevertConfirmation(): void {
        if (submittingRef.current) {
            return;
        }
        setRevertAsking(false);
        setRevertError(null);
    }

    function syncInlineRevertPosition(): void {
        if (mobile || isNil(revertItemId)) {
            return;
        }
        const wrap = wrapRef.current;
        const bubble = resolveRevertBubble(revertItemId);
        if (!wrap || !bubble) {
            return;
        }
        const box = resolveInlineRevertBox(wrap, bubble, null);
        if (!box) {
            return;
        }
        setFollowupHeight(Math.round(wrap.getBoundingClientRect().height));
        setInlineRevertBox(box);
    }

    function confirmRevert(): void {
        if (submittingRef.current) {
            return;
        }
        if (isEmpty(chatId)) {
            return;
        }
        if (isNil(revertItemId)) {
            return;
        }
        const target = resolveEditor();
        if (!target) {
            return;
        }
        const message = serializeEditorMessage(target).trim();
        submittingRef.current = true;
        setRevertSubmitting(true);
        aiChatService.revert(chatId, revertItemId).then(() => {
            sendChat(chatId, target, message);
            if (chatIdRef.current !== chatId) {
                return;
            }
            isRevertingRef.current = false;
            preRevertContentRef.current = null;
            setRevertItemId(null);
            setRevertAsking(false);
            clearInlineRevertLayout();
            onReverted();
        }).catch((err) => {
            if (chatIdRef.current === chatId) {
                setRevertError(resolveErrorText(err));
            } else {
                console.error('[input] resubmit failed', err);
            }
        }).finally(() => {
            submittingRef.current = false;
            setRevertSubmitting(false);
        });
    }

    useEffect(() => {
        const draft = aiChatService.getInputCache(chatId);
        const editing = draft === null ? undefined : draft.revert;
        isRevertingRef.current = editing !== undefined;
        preRevertContentRef.current = editing === undefined ? null : editing.originalContent;
        setRevertItemId(editing === undefined ? null : editing.itemId);
        setRevertAsking(false);
        setRevertError(null);
        clearInlineRevertLayout();
    }, [aiChatService, chatId]);

    useEffect(() => {
        if (!onRevertChange) {
            return;
        }
        onRevertChange(revertItemId);
    }, [onRevertChange, revertItemId]);

    useLayoutEffect(() => {
        if (isNil(revertItemId)) {
            return;
        }
        if (mobile) {
            const target = resolveEditor();
            if (target) {
                target.view.dom.focus({preventScroll: true});
            }
            return;
        }
        const wrap = wrapRef.current;
        const followup = followupRef.current;
        const bubble = resolveRevertBubble(revertItemId);
        if (!wrap || !bubble) {
            return;
        }
        const nextHeight = Math.round(wrap.getBoundingClientRect().height);
        if (followup) {
            followup.style.height = `${nextHeight}px`;
        }
        if (followupHeight !== nextHeight) {
            setFollowupHeight(nextHeight);
        }
        const box = ensureInlineRevertInView(wrap, bubble, followupRef.current);
        if (box) {
            setInlineRevertBox(box);
        }
        const target = resolveEditor();
        if (target) {
            target.view.dom.focus({preventScroll: true});
        }
    }, [mobile, revertItemId, followupHeight]);

    useEffect(() => {
        if (isNil(revertItemId)) {
            return;
        }
        const body = document.querySelector('[data-tm-message-body="true"]');

        function onScroll(): void {
            syncInlineRevertPosition();
        }

        function onResize(): void {
            syncInlineRevertPosition();
        }

        const observer = new MutationObserver(syncInlineRevertPosition);
        if (body instanceof HTMLElement) {
            body.addEventListener('scroll', onScroll, {passive: true});
            if (!mobile) {
                observer.observe(body, {childList: true, subtree: true});
            }
        }
        window.addEventListener('resize', onResize);
        return () => {
            observer.disconnect();
            if (body instanceof HTMLElement) {
                body.removeEventListener('scroll', onScroll);
            }
            window.removeEventListener('resize', onResize);
        };
    }, [revertItemId, chatId, mobile]);

    return {revertItemId, revertAsking, revertSubmitting, revertError, inlineRevertBox, followupHeight, followupRef, isRevertingRef, beginRevert, cancelRevert, dismissRevertConfirmation, confirmRevert, askRevert};
}
