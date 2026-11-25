import type {Editor} from '@tiptap/react';
import {RefObject, useEffect, useRef} from 'react';
import type {ChatReference} from '@/common/model/dto/ChatReference.ts';
import {insertEditorReference} from '@/page/common/util/reference/editor.ts';
import {registerReferenceDropZone} from '@/page/common/util/reference/drag.ts';

export interface UseReferenceDropZoneParams {
    sessionId: string;
    editor: Editor | null;
    resolveEditor: () => Editor | null;
    setDropOver: (over: boolean) => void;
    panelRef: RefObject<HTMLDivElement | null>;
}

export function useReferenceDropZone(params: UseReferenceDropZoneParams): void {
    const dragCaretRef = useRef<number | null>(null);
    const {sessionId, editor, panelRef, resolveEditor, setDropOver} = params;

    useEffect(() => {
        const panel = panelRef.current;
        if (!panel) {
            return;
        }
        return registerReferenceDropZone({
            el: panel,
            onOver: () => {},
            onEnter: () => {
                const target = resolveEditor();
                if (target) {
                    dragCaretRef.current = target.state.selection.from;
                }
                setDropOver(true);
            },
            onLeave: () => setDropOver(false),
            onDrop: (ref: ChatReference) => {
                const target = resolveEditor();
                if (!target) {
                    return;
                }
                insertEditorReference(target, ref, dragCaretRef.current);
                dragCaretRef.current = null;
            },
        });
    }, [editor, sessionId]);
}
