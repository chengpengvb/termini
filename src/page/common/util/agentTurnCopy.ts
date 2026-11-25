import {isNil, isNotNil} from '@/common/util/StringUtil.ts';
import type {AiChatItem} from '@/common/model/AiChatItem.ts';
import {isSameDay} from '@/page/common/util/agentTimeFormat.ts';
import {resolveResultMessage} from '@/common/model/resp/ResultModel.ts';
import {AI_TOOL_ID, resolveAiToolId} from '@/common/const/AiToolConst.ts';
import {RESP_EVENT_AGENT} from '@/common/model/resp/event/RespEventAgent.ts';

export class AgentTurnCopyBuffer {
    private readonly texts = new Map<string, string>();

    clear(): void {
        this.texts.clear();
    }

    accumulate(item: AiChatItem): void {
        const key = item.pid ? item.pid : item.id;
        if (item.type === RESP_EVENT_AGENT.ON_USER) {
            const message = item.context.message;
            if (message) {
                this.texts.set(key, message);
            }
            return;
        }
        if (item.type === RESP_EVENT_AGENT.ON_TEXT || item.type === RESP_EVENT_AGENT.ON_REASONING || item.type === RESP_EVENT_AGENT.ON_TOOL_CALL_CHUNK) {
            const delta = item.context.delta;
            if (isNil(delta)) {
                return;
            }
            const prev = this.texts.get(key);
            this.texts.set(key, isNil(prev) ? delta : prev + delta);
            return;
        }
        if (item.type === RESP_EVENT_AGENT.ON_TEXT_FINAL || item.type === RESP_EVENT_AGENT.ON_REASONING_FINAL) {
            this.texts.set(key, item.context.delta);
            return;
        }
        if (item.type === RESP_EVENT_AGENT.ON_TOOL_CALL_DONE) {
            this.texts.set(key, item.context.result);
            return;
        }
        if (item.type === RESP_EVENT_AGENT.ON_ERROR) {
            const code = item.context.code;
            if (isNotNil(code)) {
                this.texts.set(key, resolveResultMessage(code));
                return;
            }
            const error = item.context.error;
            if (error) {
                this.texts.set(key, error);
            }
        }
    }

    resolveItemText(itemId: string): string | null {
        const text = this.texts.get(itemId);
        if (isNil(text)) {
            return null;
        }
        return text;
    }
}

export interface AgentMessageRow {
    key: string;
    turnIndex: number;
    turn: AiChatItem[];
    dayDivider?: number;
    item: AiChatItem | null;
    taskItems?: AiChatItem[];
    groupItems?: AiChatItem[];
    writeItems?: AiChatItem[];
    processRows?: AgentMessageRow[];

}

function isConclusionItem(item: AiChatItem): boolean {
    if (item.type === RESP_EVENT_AGENT.ON_TEXT_FINAL || item.type === RESP_EVENT_AGENT.ON_TEXT || item.type === RESP_EVENT_AGENT.ON_IMAGE || item.type === RESP_EVENT_AGENT.ON_ERROR) {
        return true;
    }
    return item.type === RESP_EVENT_AGENT.ON_TOOL_CALL && resolveAiToolId(item.context.toolId) === AI_TOOL_ID.ASK_USER;
}

function foldProcessRows(turnRows: AgentMessageRow[], turn: AiChatItem[], turnIndex: number): AgentMessageRow[] {
    const start = turnRows.length > 0 && isNotNil(turnRows[0].item) && turnRows[0].item.type === RESP_EVENT_AGENT.ON_USER ? 1 : 0;
    let end = turnRows.length;
    while (end > start) {
        const row = turnRows[end - 1];
        if (isNil(row.item) || !isConclusionItem(row.item)) {
            break;
        }
        end--;
    }
    if (end - start <= 0) {
        return turnRows;
    }
    const processRow: AgentMessageRow = {key: `${turn[0].id}#process`, item: null, processRows: turnRows.slice(start, end), turn, turnIndex};
    return [...turnRows.slice(0, start), processRow, ...turnRows.slice(end)];
}

function isTaskRoot(item: AiChatItem): boolean {
    return item.type === RESP_EVENT_AGENT.ON_TOOL_CALL && resolveAiToolId(item.context.toolId) === AI_TOOL_ID.TASK;
}

const CONTEXT_QUERY_TOOL_IDS = new Set<number>([
    AI_TOOL_ID.CHAT_HISTORY,
    AI_TOOL_ID.DEVICE_LIST,
    AI_TOOL_ID.GROUP_LIST,
    AI_TOOL_ID.SKILL_LIST,
    AI_TOOL_ID.DERIVED_LIST,
    AI_TOOL_ID.TASK_LIST,
    AI_TOOL_ID.JOB_INDICATOR_LIST,
]);

function isContextQueryRoot(item: AiChatItem): boolean {
    if (item.type !== RESP_EVENT_AGENT.ON_TOOL_CALL) {
        return false;
    }
    const toolId = resolveAiToolId(item.context.toolId);
    return isNotNil(toolId) && CONTEXT_QUERY_TOOL_IDS.has(toolId);
}

const FILE_WRITE_TOOL_IDS = new Set<number>([
    AI_TOOL_ID.SEARCH_REPLACE,
    AI_TOOL_ID.SFTP_SEARCH_REPLACE,
    AI_TOOL_ID.DOCX_WRITE,
    AI_TOOL_ID.PDF_WRITE,
    AI_TOOL_ID.XLSX_WRITE,
]);

function isFileWriteRoot(item: AiChatItem): boolean {
    if (item.type !== RESP_EVENT_AGENT.ON_TOOL_CALL) {
        return false;
    }
    const toolId = resolveAiToolId(item.context.toolId);
    return isNotNil(toolId) && FILE_WRITE_TOOL_IDS.has(toolId);
}

function resolveTurnStartAt(turn: AiChatItem[]): number {
    return turn[0].timestamp;
}

function buildTurnRows(turn: AiChatItem[], turnIndex: number): AgentMessageRow[] {
    const rows: AgentMessageRow[] = [];
    let index = 0;
    while (index < turn.length) {
        const item = turn[index];
        if (isContextQueryRoot(item)) {
            let end = index + 1;
            while (end < turn.length && isContextQueryRoot(turn[end])) {
                end++;
            }
            if (end - index >= 2) {
                rows.push({key: `${item.id}#ctxgroup`, item: null, groupItems: turn.slice(index, end), turn, turnIndex});
                index = end;
                continue;
            }
        }
        if (isFileWriteRoot(item)) {
            let end = index + 1;
            while (end < turn.length && isFileWriteRoot(turn[end])) {
                end++;
            }
            if (end - index >= 2) {
                rows.push({key: `${item.id}#writegroup`, item: null, writeItems: turn.slice(index, end), turn, turnIndex});
                index = end;
                continue;
            }
        }
        if (isTaskRoot(item)) {
            let end = index + 1;
            while (end < turn.length && isTaskRoot(turn[end])) {
                end++;
            }
            rows.push({key: `${item.id}#taskgroup`, item: null, taskItems: turn.slice(index, end), turn, turnIndex});
            index = end;
            continue;
        }
        rows.push({key: item.id, item, turn, turnIndex});
        index++;
    }
    return rows;
}

export function flattenAgentRows(turns: AiChatItem[][], isTurnEnded: (turnIndex: number) => boolean): AgentMessageRow[] {
    const rows: AgentMessageRow[] = [];
    for (let turnIndex = 0; turnIndex < turns.length; turnIndex++) {
        const turn = turns[turnIndex];
        const startAt = resolveTurnStartAt(turn);
        if (startAt > 0 && (turnIndex === 0 || !isSameDay(resolveTurnStartAt(turns[turnIndex - 1]), startAt))) {
            rows.push({key: `${turn[0].id}#day`, item: null, turn, turnIndex, dayDivider: startAt});
        }
        const turnRows = buildTurnRows(turn, turnIndex);
        rows.push(...(isTurnEnded(turnIndex) ? foldProcessRows(turnRows, turn, turnIndex) : turnRows));
        rows.push({key: `${turn[0].id}#tail`, item: null, turn, turnIndex});
    }
    return rows;
}

export function groupAgentTurns(items: AiChatItem[]): AiChatItem[][] {
    const turns: AiChatItem[][] = [];
    for (const item of items) {
        if (item.type === RESP_EVENT_AGENT.ON_USER || turns.length === 0) {
            turns.push([item]);
            continue;
        }
        turns[turns.length - 1].push(item);
    }
    return turns;
}

export function buildAgentTurnText(turn: AiChatItem[], buffer: AgentTurnCopyBuffer): string {
    const parts: string[] = [];
    for (const item of turn) {
        const text = buffer.resolveItemText(item.id);
        if (isNil(text)) {
            continue;
        }
        const trimmed = text.trim();
        if (!trimmed) {
            continue;
        }
        parts.push(trimmed);
    }
    return parts.join('\n\n');
}

export function canCopyAgentTurn(index: number, total: number, runDone: boolean): boolean {
    if (index < total - 1) {
        return true;
    }
    return runDone;
}
