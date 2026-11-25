import type {Editor} from '@tiptap/react';
import type {Device} from '@/common/model/Device.ts';
import type {AiSkill} from '@/common/model/AiSkill.ts';
import type {RespTree} from '@/common/model/resp/RespTree.ts';
import {CHAT_TARGET_TYPE} from '@/common/const/ChatTargetConst.ts';
import {isEmpty, isNil, isNotEmpty} from '@/common/util/StringUtil.ts';
import type {ChatReference} from '@/common/model/dto/ChatReference.ts';
import {serializeEditorMessage} from '@/page/common/util/reference/editor.ts';
import type {McpPromptSnapshot, McpToolSnapshot} from '@/common/model/AiMcpServer.ts';

export const HINT_TRIGGER = {
    SLASH: '/',
    MENTION: '@',
} as const;

export const HINT_GROUP_VISIBLE = 3;

const ATOM_TEXT = '\u0000';
const MENTION_REGEX = /(?:^|[\s\u0000])@([^\s@\u0000]*)$/;

export interface HintServer {
    id: string;
    name: string;
}

export interface HintItem {
    id: string;
    kind: number;
    name: string;
    data: any | null;
    server: HintServer | null;
    description: string | null;
}

export const HINT_SERVERS_KEY = 'servers';

export interface HintGroup {
    key: string;
    kind: number;
    total: number;
    items: HintItem[];
    moreCount: number;
    collapsed: boolean;
    server: HintServer | null;
    description: string | null;
    serversMore: boolean;
}

export const HINT_ROW = {
    ITEM: 0,
    MORE: 1,
    SERVER: 2,
} as const;

export interface HintRow {
    type: number;
    group: HintGroup;
    item: HintItem | null;
}

export interface HintRange {
    from: number;
    to: number;
}

export interface HintQuery {
    query: string;
    trigger: string;
    range: HintRange | null;
}

function resolveSlashQuery(editor: Editor): HintQuery | null {
    const text = serializeEditorMessage(editor);
    if (!text.startsWith(HINT_TRIGGER.SLASH) || text.includes('\n')) {
        return null;
    }
    return {trigger: HINT_TRIGGER.SLASH, query: text.slice(1), range: null};
}

function resolveMentionQuery(editor: Editor): HintQuery | null {
    const {empty, from, $from} = editor.state.selection;
    if (!empty) {
        return null;
    }
    const before = $from.parent.textBetween(0, $from.parentOffset, undefined, ATOM_TEXT);
    const match = MENTION_REGEX.exec(before);
    if (match === null) {
        return null;
    }
    const query = match[1];
    return {trigger: HINT_TRIGGER.MENTION, query, range: {from: from - query.length - 1, to: from}};
}

export function resolveHintQuery(editor: Editor): HintQuery | null {
    if (editor.isEmpty) {
        return null;
    }
    const slash = resolveSlashQuery(editor);
    if (slash !== null) {
        return slash;
    }
    return resolveMentionQuery(editor);
}

export function buildSkillHintItems(skills: AiSkill[]): HintItem[] {
    const items: HintItem[] = [];
    for (const skill of skills) {
        if (isEmpty(skill.id) || isEmpty(skill.name)) {
            continue;
        }
        items.push({id: skill.id, kind: CHAT_TARGET_TYPE.SKILL, name: skill.name, data: null, server: null, description: skill.description});
    }
    return items;
}

export function buildToolHintItems(tools: McpToolSnapshot[]): HintItem[] {
    return tools.map(tool => ({id: `${tool.server_id}/${tool.name}`, kind: CHAT_TARGET_TYPE.MCP_TOOL, name: tool.name, data: null, server: {id: tool.server_id, name: tool.server_name}, description: tool.description}));
}

export function buildPromptHintItems(prompts: McpPromptSnapshot[]): HintItem[] {
    return prompts.map(prompt => ({id: `${prompt.server_id}/${prompt.name}`, kind: CHAT_TARGET_TYPE.MCP_PROMPT, name: prompt.name, data: null, server: {id: prompt.server_id, name: prompt.server_name}, description: prompt.description}));
}

function collectTreeHintItems(nodes: RespTree[], devices: HintItem[], groups: HintItem[]): void {
    for (const node of nodes) {
        if (isNotEmpty(node.title) && node.type === CHAT_TARGET_TYPE.GROUP) {
            groups.push({id: node.id, kind: CHAT_TARGET_TYPE.GROUP, name: node.title, data: null, server: null, description: null});
        }
        if (isNotEmpty(node.title) && node.type === CHAT_TARGET_TYPE.DEVICE && isNotEmpty(node.data)) {
            const device: Device = JSON.parse(node.data);
            devices.push({id: node.id, kind: CHAT_TARGET_TYPE.DEVICE, name: node.title, data: device, server: null, description: device.host});
        }
        if (!isNil(node.children)) {
            collectTreeHintItems(node.children, devices, groups);
        }
    }
}

export function buildTreeHintItems(tree: RespTree[]): HintItem[] {
    const devices: HintItem[] = [];
    const groups: HintItem[] = [];
    collectTreeHintItems(tree, devices, groups);
    return [...devices, ...groups];
}

export function filterHintItems(items: HintItem[], query: string): HintItem[] {
    const value = query.trim().toLowerCase();
    if (isEmpty(value)) {
        return items;
    }
    return items.filter(item => item.name.toLowerCase().includes(value));
}

export interface HintFold {
    expanded: string[];
    collapsed: string[];
}

function pushKindGroup(groups: HintGroup[], matched: HintItem[], kind: number, fold: HintFold): void {
    const list = matched.filter(item => item.kind === kind);
    if (list.length === 0) {
        return;
    }
    const key = `kind:${kind}`;
    if (fold.expanded.includes(key) || list.length <= HINT_GROUP_VISIBLE) {
        groups.push({key, kind, server: null, description: null, serversMore: false, total: list.length, items: list, moreCount: 0, collapsed: false});
        return;
    }
    groups.push({key, kind, server: null, description: null, serversMore: false, total: list.length, items: list.slice(0, HINT_GROUP_VISIBLE), moreCount: list.length - HINT_GROUP_VISIBLE, collapsed: false});
}

function pushServerSection(groups: HintGroup[], matched: HintItem[], fold: HintFold): void {
    const seen = new Set<string>();
    const servers: {server: HintServer; list: HintItem[]}[] = [];
    for (const item of matched) {
        if (item.server === null || seen.has(item.server.id)) {
            continue;
        }
        seen.add(item.server.id);
        const server = item.server;
        servers.push({server, list: matched.filter(other => other.server !== null && other.server.id === server.id)});
    }
    if (servers.length === 0) {
        return;
    }
    const showAll = fold.expanded.includes(HINT_SERVERS_KEY);
    const visible = showAll ? servers : servers.slice(0, HINT_GROUP_VISIBLE);
    for (const {server, list} of visible) {
        const key = `server:${server.id}`;
        const open = fold.collapsed.includes(key);
        groups.push({
            key,
            kind: CHAT_TARGET_TYPE.MCP_TOOL,
            server,
            description: list.map(entry => entry.name).join(', '),
            serversMore: false,
            total: list.length,
            items: open ? list : [],
            moreCount: 0,
            collapsed: !open,
        });
    }
    const rest = servers.length - visible.length;
    if (rest > 0) {
        groups.push({key: HINT_SERVERS_KEY, kind: CHAT_TARGET_TYPE.MCP_TOOL, server: null, description: null, serversMore: true, total: servers.length, items: [], moreCount: rest, collapsed: false});
    }
}

export function buildHintGroups(matched: HintItem[], fold: HintFold): HintGroup[] {
    const groups: HintGroup[] = [];
    pushKindGroup(groups, matched, CHAT_TARGET_TYPE.SKILL, fold);
    pushServerSection(groups, matched, fold);
    pushKindGroup(groups, matched, CHAT_TARGET_TYPE.DEVICE, fold);
    pushKindGroup(groups, matched, CHAT_TARGET_TYPE.GROUP, fold);
    return groups;
}

export function flattenHintRows(groups: HintGroup[]): HintRow[] {
    const rows: HintRow[] = [];
    for (const group of groups) {
        if (group.server !== null) {
            rows.push({type: HINT_ROW.SERVER, group, item: null});
        }
        for (const item of group.items) {
            rows.push({type: HINT_ROW.ITEM, group, item});
        }
        if (group.moreCount > 0) {
            rows.push({type: HINT_ROW.MORE, group, item: null});
        }
    }
    return rows;
}

export function hintItemReference(item: HintItem): ChatReference {
    return {kind: item.kind, id: item.id, title: item.name, data: item.data};
}

export function hintServerReference(server: HintServer): ChatReference {
    return {kind: CHAT_TARGET_TYPE.MCP_SERVER, id: server.id, title: server.name, data: null};
}

export function buildSlashHintItems(skills: AiSkill[], tools: McpToolSnapshot[], prompts: McpPromptSnapshot[]): HintItem[] {
    return [...buildSkillHintItems([...skills].sort((a, b) => a.sort - b.sort)), ...buildToolHintItems(tools), ...buildPromptHintItems(prompts)];
}
