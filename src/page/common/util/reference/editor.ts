import Text from '@tiptap/extension-text';
import type {Editor} from '@tiptap/react';
import {translate} from '@/common/i18n/i18n.ts';
import Document from '@tiptap/extension-document';
import Paragraph from '@tiptap/extension-paragraph';
import {isNotNil} from '@/common/util/StringUtil.ts';
import HardBreak from '@tiptap/extension-hard-break';
import {Placeholder, UndoRedo} from '@tiptap/extensions';
import {ReferenceNode} from '@/page/common/util/reference/node.ts';
import type {ChatReference} from '@/common/model/dto/ChatReference.ts';
import {MESSAGE_PART, serializeReferenceToken, splitMessageParts} from '@/page/common/util/reference/token.ts';

interface ReferenceEditorOptions {
    content?: object;
    onSubmit: () => void;
    placeholder: string | (() => string);
    onUpdate: (editor: Editor) => void;
    onArrow?: (delta: number) => boolean;
}

function resolveRefNode(inner: Element): HTMLElement {
    const parent = inner.parentElement;
    if (!(parent instanceof HTMLElement)) {
        throw new Error('reference wrap parent missing');
    }
    parent.setAttribute('data-tm-ref-node', 'true');
    return parent;
}

function markAdjacentReferenceWraps(root: ParentNode): void {
    root.querySelectorAll('[data-tm-ref-wrap="true"]').forEach((inner) => {
        const el = resolveRefNode(inner);
        const prev = el.previousSibling;
        el.classList.toggle('is-adj', prev instanceof Element && prev.getAttribute('data-tm-ref-node') === 'true');
    });
}

function markSelectedReferenceWraps(root: ParentNode): void {
    const sel = window.getSelection();
    if (sel === null) {
        throw new Error('window.getSelection() is null');
    }
    if (sel.rangeCount === 0 || sel.isCollapsed) {
        root.querySelectorAll('[data-tm-ref-wrap="true"]').forEach((inner) => {
            resolveRefNode(inner).classList.remove('is-sel');
        });
        return;
    }
    const range = sel.getRangeAt(0);
    root.querySelectorAll('[data-tm-ref-wrap="true"]').forEach((inner) => {
        const el = resolveRefNode(inner);
        el.classList.toggle('is-sel', range.intersectsNode(el));
    });
}

export function createReferenceEditorOptions(options: ReferenceEditorOptions) {
    const {placeholder, onSubmit, onUpdate, onArrow, content} = options;
    let adjObserver: MutationObserver | null = null;
    let onSelectionChange: (() => void) | null = null;
    return {
        content,
        extensions: [
            Text,
            Document,
            Paragraph,
            HardBreak,
            UndoRedo,
            ReferenceNode,
            Placeholder.configure({placeholder}),
        ],
        editorProps: {
            attributes: {
                class: 'tm-agent-input-editor',
                role: 'textbox',
                'aria-label': translate('common.message'),
            },
            handleKeyDown: (_view: unknown, event: KeyboardEvent) => {
                if (event.isComposing) {
                    return false;
                }
                if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
                    if (onArrow === undefined || !onArrow(event.key === 'ArrowDown' ? 1 : -1)) {
                        return false;
                    }
                    event.preventDefault();
                    return true;
                }
                if (event.key !== 'Enter' || event.shiftKey) {
                    return false;
                }
                event.preventDefault();
                onSubmit();
                return true;
            },
        },
        onCreate: ({editor}: {editor: Editor}) => {
            const root = editor.view.dom;
            const markAdj = () => {
                markAdjacentReferenceWraps(root);
            };
            const markSel = () => {
                markSelectedReferenceWraps(root);
            };
            markAdj();
            markSel();
            adjObserver = new MutationObserver(markAdj);
            adjObserver.observe(root, {childList: true, subtree: true});
            onSelectionChange = markSel;
            document.addEventListener('selectionchange', onSelectionChange);
        },
        onDestroy: () => {
            if (adjObserver !== null) {
                adjObserver.disconnect();
                adjObserver = null;
            }
            if (onSelectionChange !== null) {
                document.removeEventListener('selectionchange', onSelectionChange);
                onSelectionChange = null;
            }
        },
        onSelectionUpdate: ({editor}: {editor: Editor}) => {
            markSelectedReferenceWraps(editor.view.dom);
            if (!editor.isEmpty) {
                return;
            }
            if (editor.state.selection.empty) {
                return;
            }
            editor.commands.setTextSelection(editor.state.selection.from);
        },
        onUpdate: ({editor}: {editor: Editor}) => {
            onUpdate(editor);
        },
    };
}

function buildTextNodes(text: string): object[] {
    const nodes: object[] = [];
    text.split('\n').forEach((line, index) => {
        if (index > 0) {
            nodes.push({type: 'hardBreak'});
        }
        if (line.length > 0) {
            nodes.push({type: 'text', text: line});
        }
    });
    return nodes;
}

export function serializeEditorMessage(editor: Editor): string {
    return editor.getText({
        blockSeparator: '\n',
        textSerializers: {
            reference: ({node}) => serializeReferenceToken(node.attrs.kind, node.attrs.id),
        },
    });
}

export function insertEditorReference(editor: Editor, ref: ChatReference, pos: number | null): void {
    const chain = editor.chain().focus(null, {scrollIntoView: false});
    if (isNotNil(pos)) {
        chain.setTextSelection(pos);
    }
    chain.insertReference({kind: ref.kind, id: ref.id, title: ref.title, data: ref.data}).run();
    editor.view.dom.focus({preventScroll: true});
}

export function focusEditor(editor: Editor): void {
    editor.chain().focus('start', {scrollIntoView: false}).run();
    editor.view.dom.focus({preventScroll: true});
}

export function setEditorMessage(editor: Editor, text: string): void {
    const content = splitMessageParts(text).flatMap((part) => {
        if (part.part === MESSAGE_PART.REFERENCE) {
            return [{type: 'reference', attrs: {kind: part.token.kind, id: part.token.id, title: part.token.id, data: null}}];
        }
        return buildTextNodes(part.text);
    });
    editor.commands.setContent({type: 'doc', content: [{type: 'paragraph', content}]});
    editor.commands.focus('end', {scrollIntoView: false});
    editor.view.dom.focus({preventScroll: true});
}

export function clearEditorContent(editor: Editor): void {
    editor.commands.clearContent(true);
}
