import {createElement} from 'react';
import type {ReactNode} from 'react';
import type {NodeViewProps} from '@tiptap/react';
import {isEmpty} from '@/common/util/StringUtil.ts';
import {Node, nodeInputRule, nodePasteRule} from '@tiptap/core';
import {CHAT_TARGET_TYPE} from '@/common/const/ChatTargetConst.ts';
import {NodeViewWrapper, ReactNodeViewRenderer} from '@tiptap/react';
import {ReferenceChip} from '@/page/common/util/reference/ReferenceChip.tsx';
import {keyToKind, resolveLocalPathName, serializeReferenceToken, TOKEN_REGEX} from '@/page/common/util/reference/token.ts';

const INPUT_REGEX = /@\[([a-z]+):([^\]]+)]$/;

const NODE_NAME = 'reference';

interface ReferenceAttrs {
    kind: number;
    id: string;
    title: string;
    data: any | null;
}

declare module '@tiptap/core' {
    interface Commands<ReturnType> {
        reference: {
            insertReference: (attrs: ReferenceAttrs) => ReturnType;
            insertReferenceAt: (pos: number, attrs: ReferenceAttrs) => ReturnType;
        };
    }
}

function parseKindAttr(value: string | null): number | null {
    if (isEmpty(value)) {
        return null;
    }
    const parsed = Number(value);
    if (!Number.isFinite(parsed)) {
        return null;
    }
    return parsed;
}

function parseJsonDataAttr(value: string | null): any | null {
    if (isEmpty(value)) {
        return null;
    }
    try {
        return JSON.parse(value);
    } catch {
        return null;
    }
}

function resolvePasteAttrs(match: RegExpMatchArray): ReferenceAttrs | null {
    const key = match[1];
    const id = match[2];
    if (isEmpty(key)) {
        return null;
    }
    if (isEmpty(id)) {
        return null;
    }
    const kind = keyToKind(key);
    if (kind === null) {
        return null;
    }
    if (kind === CHAT_TARGET_TYPE.LOCAL_PATH) {
        return {kind, id, title: resolveLocalPathName(id), data: null};
    }
    return {kind, id, title: id, data: null};
}

function renderChipNodeView(props: NodeViewProps): ReactNode {
    const {node, deleteNode} = props;
    return createElement(NodeViewWrapper, {as: 'span', className: 'tm-agent-input-ref-nodeview', 'data-tm-ref-wrap': 'true'},
        createElement(ReferenceChip, {kind: node.attrs.kind, id: node.attrs.id, interactive: true, onRemove: deleteNode}),
    );
}

export const ReferenceNode = Node.create({
    name: NODE_NAME,
    group: 'inline',
    inline: true,
    atom: true,
    selectable: true,
    draggable: false,

    addAttributes() {
        return {
            kind: {
                default: 0,
                parseHTML: element => parseKindAttr(element.getAttribute('data-kind')),
                renderHTML: attrs => ({'data-kind': String(attrs.kind)}),
            },
            id: {
                default: '',
                parseHTML: element => element.getAttribute('data-id'),
                renderHTML: attrs => ({'data-id': attrs.id}),
            },
            title: {
                default: '',
                parseHTML: element => element.getAttribute('data-title'),
                renderHTML: attrs => ({'data-title': attrs.title}),
            },
            data: {
                default: null,
                parseHTML: element => parseJsonDataAttr(element.getAttribute('data-payload')),
                renderHTML: attrs => {
                    if (!attrs.data) {
                        return {};
                    }
                    return {'data-payload': JSON.stringify(attrs.data)};
                },
            },
        };
    },

    parseHTML() {
        return [
            {
                tag: 'span[data-reference]',
            },
        ];
    },

    renderHTML({node, HTMLAttributes}) {
        const merged = {
            ...HTMLAttributes,
            'data-reference': '1',
            class: 'tm-agent-input-ref-chip',
        };
        return ['span', merged, serializeReferenceToken(node.attrs.kind, node.attrs.id)];
    },

    renderText({node}) {
        return serializeReferenceToken(node.attrs.kind, node.attrs.id);
    },

    addNodeView() {
        return ReactNodeViewRenderer(renderChipNodeView);
    },

    addCommands() {
        return {
            insertReference: (attrs) => ({commands}) => commands.insertContent({type: NODE_NAME, attrs}),
            insertReferenceAt: (pos, attrs) => ({commands}) => commands.insertContentAt(pos, {type: NODE_NAME, attrs}),
        };
    },

    addInputRules() {
        return [
            nodeInputRule({
                find: INPUT_REGEX,
                type: this.type,
                getAttributes: match => resolvePasteAttrs(match),
            }),
        ];
    },

    addPasteRules() {
        return [
            nodePasteRule({
                find: TOKEN_REGEX,
                type: this.type,
                getAttributes: match => resolvePasteAttrs(match),
            }),
        ];
    },
});
