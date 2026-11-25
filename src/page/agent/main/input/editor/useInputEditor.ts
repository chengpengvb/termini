import {useEditor} from '@tiptap/react';
import type {Editor} from '@tiptap/react';
import {useAiChatService} from '@/common/context/useAppHooks.tsx';
import {addLanguageListener, translate} from '@/common/i18n/i18n.ts';
import type {ChatReference} from '@/common/model/dto/ChatReference.ts';
import {RefObject, useEffect, useRef, useState, type MouseEvent as ReactMouseEvent} from 'react';
import {clearEditorContent, createReferenceEditorOptions, focusEditor, insertEditorReference, setEditorMessage} from '@/page/common/util/reference/editor.ts';

export interface UseInputEditorParams {
    onSubmit: () => void;
    chatId: string | null;
    chatIdRef: RefObject<string | null>;
    onArrow: (delta: number) => boolean;
    onEditorUpdate: (editor: Editor) => void;
}

export function useInputEditor(params: UseInputEditorParams) {
    const paramsRef = useRef(params);
    const {chatId, chatIdRef} = params;
    const aiChatService = useAiChatService();
    const cached = aiChatService.getInputCache(chatId);
    const editorInstanceRef = useRef<Editor | null>(null);
    const [hasAny, setHasAny] = useState<boolean>(cached !== null && cached.content !== null);

    paramsRef.current = params;

    const editor = useEditor(
        createReferenceEditorOptions({
            placeholder: () => translate('agent.empty.askPlaceholder'),
            content: cached === null || cached.content === null ? undefined : cached.content,
            onSubmit: () => {
                paramsRef.current.onSubmit();
            },
            onArrow: delta => paramsRef.current.onArrow(delta),
            onUpdate: target => {
                setHasAny(!target.isEmpty);
                paramsRef.current.onEditorUpdate(target);
                const previous = aiChatService.getInputCache(chatIdRef.current);
                aiChatService.setInputCache(chatIdRef.current, {...previous, content: target.isEmpty ? null : target.getJSON()});
            },
        }),
    );

    editorInstanceRef.current = editor;

    useEffect(() => {
        return addLanguageListener(() => {
            const target = editorInstanceRef.current;
            if (!target) {
                return;
            }
            target.view.dispatch(target.state.tr);
        });
    }, []);

    function resolveEditor(): Editor | null {
        return editorInstanceRef.current;
    }

    function loadCache(nextChatId: string | null): void {
        const target = resolveEditor();
        if (!target) {
            return;
        }
        const next = aiChatService.getInputCache(nextChatId);
        if (next === null || next.content === null) {
            clearEditorContent(target);
        } else {
            target.commands.setContent(next.content);
        }
        setHasAny(!target.isEmpty);
    }

    function insertReference(ref: ChatReference): void {
        const target = resolveEditor();
        if (!target) {
            return;
        }
        insertEditorReference(target, ref, null);
    }

    function applyReference(ref: ChatReference): void {
        const target = resolveEditor();
        if (!target) {
            return;
        }
        target.chain().focus(null, {scrollIntoView: false}).clearContent().run();
        target.view.dom.focus({preventScroll: true});
        insertEditorReference(target, ref, null);
    }

    function restoreDraft(message: string): void {
        const target = resolveEditor();
        if (!target || !target.isEmpty) {
            return;
        }
        setEditorMessage(target, message);
        setHasAny(!target.isEmpty);
        focusEditor(target);
    }

    function handlePanelClick(e: ReactMouseEvent<HTMLDivElement>): void {
        if (e.target !== e.currentTarget) {
            return;
        }
        const target = resolveEditor();
        if (!target) {
            return;
        }
        focusEditor(target);
    }

    return {editor, hasAny, setHasAny, resolveEditor, loadCache, insertReference, applyReference, restoreDraft, handlePanelClick};
}
