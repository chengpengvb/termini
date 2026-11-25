import type {ChatReference} from '@/common/model/dto/ChatReference.ts';
import {isEmpty, isNotEmpty, isNotNil} from '@/common/util/StringUtil.ts';
import {CHAT_TARGET_KEY, CHAT_TARGET_TYPE} from '@/common/const/ChatTargetConst.ts';

const KIND_TO_KEY: Record<number, string> = {
    [CHAT_TARGET_TYPE.DEVICE]: CHAT_TARGET_KEY.DEVICE,
    [CHAT_TARGET_TYPE.GROUP]: CHAT_TARGET_KEY.GROUP,
    [CHAT_TARGET_TYPE.LOCAL_PATH]: CHAT_TARGET_KEY.LOCAL_PATH,
    [CHAT_TARGET_TYPE.SKILL]: CHAT_TARGET_KEY.SKILL,
    [CHAT_TARGET_TYPE.MCP_PROMPT]: CHAT_TARGET_KEY.MCP_PROMPT,
    [CHAT_TARGET_TYPE.MCP_TOOL]: CHAT_TARGET_KEY.MCP_TOOL,
    [CHAT_TARGET_TYPE.MCP_SERVER]: CHAT_TARGET_KEY.MCP_SERVER,
};

const KEY_TO_KIND: Record<string, number> = {
    [CHAT_TARGET_KEY.DEVICE]: CHAT_TARGET_TYPE.DEVICE,
    [CHAT_TARGET_KEY.GROUP]: CHAT_TARGET_TYPE.GROUP,
    [CHAT_TARGET_KEY.LOCAL_PATH]: CHAT_TARGET_TYPE.LOCAL_PATH,
    [CHAT_TARGET_KEY.SKILL]: CHAT_TARGET_TYPE.SKILL,
    [CHAT_TARGET_KEY.MCP_PROMPT]: CHAT_TARGET_TYPE.MCP_PROMPT,
    [CHAT_TARGET_KEY.MCP_TOOL]: CHAT_TARGET_TYPE.MCP_TOOL,
    [CHAT_TARGET_KEY.MCP_SERVER]: CHAT_TARGET_TYPE.MCP_SERVER,
};

export const TOKEN_REGEX = /@\[([a-z]+):([^\]]+)]/g;

export function resolveLocalPathName(path: string): string {
    if (isEmpty(path)) {
        return '';
    }
    const trimmed = path.replace(/[\\/]+$/, '');
    const index = Math.max(trimmed.lastIndexOf('\\'), trimmed.lastIndexOf('/'));
    if (index < 0) {
        return trimmed;
    }
    return trimmed.slice(index + 1);
}

export function resolveMcpPromptName(id: string): string {
    const index = id.indexOf('/');
    if (index < 0) {
        return id;
    }
    return id.slice(index + 1);
}

export function isChatTextPath(path: string): boolean {
    if (isEmpty(path)) {
        return false;
    }
    const normalized = path.replace(/\\/g, '/');
    if (!normalized.endsWith('.txt')) {
        return false;
    }
    return /\/chat\/[^/]+\/texts\//.test(normalized);
}

export const MESSAGE_PART = {
    TEXT: 0,
    REFERENCE: 1,
} as const;

export interface TokenMatch {
    kind: number;
    id: string;
    raw: string;
    start: number;
    end: number;
}

export interface MessagePartText {
    part: 0;
    text: string;
}

export interface MessagePartReference {
    part: 1;
    token: TokenMatch;
}

export function kindToKey(kind: number): string | null {
    const key = KIND_TO_KEY[kind];
    if (isEmpty(key)) {
        return null;
    }
    return key;
}

export function keyToKind(key: string): number | null {
    if (isEmpty(key)) {
        return null;
    }
    const kind = KEY_TO_KIND[key];
    if (!isNotNil(kind)) {
        return null;
    }
    return kind;
}

export function stringifyToken(kind: number, id: string): string | null {
    if (isEmpty(id)) {
        return null;
    }
    const key = kindToKey(kind);
    if (isEmpty(key)) {
        return null;
    }
    return `@[${key}:${id}]`;
}

export function serializeReferenceToken(kind: number, id: string): string {
    const token = stringifyToken(kind, id);
    if (isEmpty(token)) {
        return id;
    }
    return token;
}

export function matchTokens(text: string): TokenMatch[] {
    if (isEmpty(text)) {
        return [];
    }
    const result: TokenMatch[] = [];
    const regex = new RegExp(TOKEN_REGEX.source, 'g');
    let match: RegExpExecArray | null = regex.exec(text);
    while (match !== null) {
        const key = match[1];
        const id = match[2];
        const kind = keyToKind(key);
        if (kind !== null && isNotEmpty(id)) {
            result.push({kind, id, raw: match[0], start: match.index, end: match.index + match[0].length});
        }
        match = regex.exec(text);
    }
    return result;
}

export function tokenToReference(token: TokenMatch, resolveTitle: (kind: number, id: string) => string | null): ChatReference {
    const title = resolveTitle(token.kind, token.id);
    if (isEmpty(title)) {
        return {kind: token.kind, id: token.id, title: token.id, data: null};
    }
    return {kind: token.kind, id: token.id, title, data: null};
}

export function splitMessageParts(text: string): (MessagePartText | MessagePartReference)[] {
    if (isEmpty(text)) {
        return [];
    }
    const tokens = matchTokens(text);
    if (tokens.length === 0) {
        return [{part: MESSAGE_PART.TEXT, text}];
    }
    const parts: (MessagePartText | MessagePartReference)[] = [];
    let cursor = 0;
    for (const token of tokens) {
        if (token.start > cursor) {
            parts.push({part: MESSAGE_PART.TEXT, text: text.slice(cursor, token.start)});
        }
        parts.push({part: MESSAGE_PART.REFERENCE, token});
        cursor = token.end;
    }
    if (cursor < text.length) {
        parts.push({part: MESSAGE_PART.TEXT, text: text.slice(cursor)});
    }
    return parts;
}
