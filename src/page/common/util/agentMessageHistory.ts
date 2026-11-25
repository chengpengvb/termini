import {isNil, isNotNil} from '@/common/util/StringUtil.ts';
import type {AiChatItem} from '@/common/model/AiChatItem.ts';
import type {AgentTurnClock} from '@/page/common/util/agentTurnClock.ts';
import {RESP_EVENT_AGENT} from '@/common/model/resp/event/RespEventAgent.ts';
import type {AgentTurnCopyBuffer} from '@/page/common/util/agentTurnCopy.ts';
import type {AgentRunSummaryBuffer} from '@/page/common/util/agentRunSummary.ts';

export interface AgentHistoryIngestResult {
    runDone: boolean;
    roots: AiChatItem[];
    rootIds: Set<string>;
    resolver: AgentRootResolver;
    historyById: Map<string, AiChatItem[]>;
    runEndByReqId: Map<string, AiChatItem>;
}

export interface AgentRootTarget {
    self: boolean;
    rootId: string;
    rootType: number;
}

const ROOT_TYPE_OF: Record<number, number> = {
    [RESP_EVENT_AGENT.ON_USER]: RESP_EVENT_AGENT.ON_USER,
    [RESP_EVENT_AGENT.ON_IMAGE]: RESP_EVENT_AGENT.ON_IMAGE,
    [RESP_EVENT_AGENT.ON_ERROR]: RESP_EVENT_AGENT.ON_ERROR,
    [RESP_EVENT_AGENT.ON_TEXT]: RESP_EVENT_AGENT.ON_TEXT_FINAL,
    [RESP_EVENT_AGENT.ON_CONFLICT]: RESP_EVENT_AGENT.ON_TOOL_CALL,
    [RESP_EVENT_AGENT.ON_TOOL_CALL]: RESP_EVENT_AGENT.ON_TOOL_CALL,
    [RESP_EVENT_AGENT.ON_TEXT_FINAL]: RESP_EVENT_AGENT.ON_TEXT_FINAL,
    [RESP_EVENT_AGENT.ON_TOOL_CALL_DONE]: RESP_EVENT_AGENT.ON_TOOL_CALL,
    [RESP_EVENT_AGENT.ON_TOOL_CALL_CHUNK]: RESP_EVENT_AGENT.ON_TOOL_CALL,
    [RESP_EVENT_AGENT.ON_REASONING]: RESP_EVENT_AGENT.ON_REASONING_FINAL,
    [RESP_EVENT_AGENT.ON_CONFLICT_ANSWERED]: RESP_EVENT_AGENT.ON_TOOL_CALL,
    [RESP_EVENT_AGENT.ON_REASONING_FINAL]: RESP_EVENT_AGENT.ON_REASONING_FINAL,
    [RESP_EVENT_AGENT.ON_TOOL_SUPERVISE]: RESP_EVENT_AGENT.ON_TOOL_CALL,
    [RESP_EVENT_AGENT.ON_QUESTION]: RESP_EVENT_AGENT.ON_TOOL_CALL,
    [RESP_EVENT_AGENT.ON_QUESTION_ANSWERED]: RESP_EVENT_AGENT.ON_TOOL_CALL,
    [RESP_EVENT_AGENT.ON_SUBAGENT_TOOL_CALL]: RESP_EVENT_AGENT.ON_TOOL_CALL,
    [RESP_EVENT_AGENT.ON_SUBAGENT_TOOL_DONE]: RESP_EVENT_AGENT.ON_TOOL_CALL,
    [RESP_EVENT_AGENT.ON_SUBAGENT_OPEN]: RESP_EVENT_AGENT.ON_TOOL_CALL,
};

function isRunEndEvent(type: number): boolean {
    return type === RESP_EVENT_AGENT.ON_DONE || type === RESP_EVENT_AGENT.ON_CANCELLED;
}

function resolveRootType(type: number): number | null {
    const rootType = ROOT_TYPE_OF[type];
    if (isNil(rootType)) {
        return null;
    }
    return rootType;
}

export class AgentRootResolver {
    private readonly rootIdOfCallId = new Map<string, string>();
    private readonly rootIdOfSubReqId = new Map<string, string>();

    clear(): void {
        this.rootIdOfCallId.clear();
        this.rootIdOfSubReqId.clear();
    }

    isNested(item: AiChatItem): boolean {
        if (isNotNil(item.context?.parentReqId)) {
            return true;
        }
        const reqId = item.context?.reqId;
        return isNotNil(reqId) && this.rootIdOfSubReqId.has(reqId);
    }

    nestedRootId(item: AiChatItem): string | null {
        const reqId = item.context?.reqId;
        if (isNil(reqId)) {
            return null;
        }
        return this.rootIdOfSubReqId.get(reqId) ?? null;
    }

    resolve(item: AiChatItem): AgentRootTarget | null {
        if (item.type === RESP_EVENT_AGENT.ON_SUBAGENT_OPEN && item.pid && item.context?.subReqId) {
            this.rootIdOfSubReqId.set(item.context.subReqId, item.pid);
        }
        const rootType = resolveRootType(item.type);
        if (isNil(rootType)) {
            return null;
        }
        const rootId = this.resolveRootId(item, rootType);
        if (isNil(rootId)) {
            return null;
        }
        this.remember(item, rootId);
        return {rootId, rootType, self: rootId === item.id};
    }

    private resolveRootId(item: AiChatItem, rootType: number): string | null {
        const reqId = item.context?.reqId;
        if (isNotNil(reqId)) {
            const nestedRoot = this.rootIdOfSubReqId.get(reqId);
            if (isNotNil(nestedRoot)) {
                return nestedRoot;
            }
        }
        if (item.pid) {
            return item.pid;
        }
        if (item.type === rootType) {
            return item.id;
        }
        const callId = item.context.callId;
        if (isNil(callId)) {
            return null;
        }
        const rootId = this.rootIdOfCallId.get(callId);
        if (isNil(rootId)) {
            return null;
        }
        return rootId;
    }

    private remember(item: AiChatItem, rootId: string): void {
        const callId = item.context.callId;
        if (isNil(callId)) {
            return;
        }
        this.rootIdOfCallId.set(callId, rootId);
    }
}

function recordHistory(historyById: Map<string, AiChatItem[]>, id: string, item: AiChatItem): void {
    const history = historyById.get(id);
    if (!history) {
        historyById.set(id, [item]);
        return;
    }
    history.push(item);
}

function compactHistory(history: AiChatItem[], isNested: (item: AiChatItem) => boolean): AiChatItem[] {
    // 子代理会话的事件原样保留顺序, 交给任务卡片自己去整理成嵌套对话; 只压缩卡片自身的事件
    const own = history.filter(item => !isNested(item));
    const nested = history.filter(item => isNested(item));
    return [...compactOwnHistory(own), ...nested];
}

function compactOwnHistory(history: AiChatItem[]): AiChatItem[] {
    let toolCall: AiChatItem | null = null;
    let toolConflict: AiChatItem | null = null;
    let toolConflictAnswered: AiChatItem | null = null;
    let toolQuestion: AiChatItem | null = null;
    let toolDone: AiChatItem | null = null;
    let textFinal: AiChatItem | null = null;
    let reasoningFinal: AiChatItem | null = null;
    let userItem: AiChatItem | null = null;
    let errorItem: AiChatItem | null = null;
    const supervises: AiChatItem[] = [];
    const others: AiChatItem[] = [];

    for (const item of history) {
        if (item.type === RESP_EVENT_AGENT.ON_TOOL_CALL) {
            toolCall = item;
            continue;
        }
        if (item.type === RESP_EVENT_AGENT.ON_CONFLICT) {
            toolConflict = item;
            continue;
        }
        if (item.type === RESP_EVENT_AGENT.ON_CONFLICT_ANSWERED) {
            toolConflictAnswered = item;
            continue;
        }
        if (item.type === RESP_EVENT_AGENT.ON_QUESTION) {
            toolQuestion = item;
            continue;
        }
        if (item.type === RESP_EVENT_AGENT.ON_TOOL_SUPERVISE) {
            supervises.push(item);
            continue;
        }
        if (item.type === RESP_EVENT_AGENT.ON_TOOL_CALL_DONE) {
            toolDone = item;
            continue;
        }
        if (item.type === RESP_EVENT_AGENT.ON_TOOL_CALL_CHUNK) {
            continue;
        }
        if (item.type === RESP_EVENT_AGENT.ON_TEXT_FINAL) {
            textFinal = item;
            continue;
        }
        if (item.type === RESP_EVENT_AGENT.ON_TEXT) {
            continue;
        }
        if (item.type === RESP_EVENT_AGENT.ON_REASONING_FINAL) {
            reasoningFinal = item;
            continue;
        }
        if (item.type === RESP_EVENT_AGENT.ON_REASONING) {
            continue;
        }
        if (item.type === RESP_EVENT_AGENT.ON_USER) {
            userItem = item;
            continue;
        }
        if (item.type === RESP_EVENT_AGENT.ON_ERROR) {
            errorItem = item;
            continue;
        }
        others.push(item);
    }

    const compacted: AiChatItem[] = [];
    if (userItem) {
        compacted.push(userItem);
    }
    if (toolCall) {
        compacted.push(toolCall);
    }
    if (toolConflict) {
        compacted.push(toolConflict);
    }
    if (toolConflictAnswered) {
        compacted.push(toolConflictAnswered);
    }
    if (toolQuestion) {
        compacted.push(toolQuestion);
    }
    for (const item of supervises) {
        compacted.push(item);
    }
    if (toolDone) {
        compacted.push(toolDone);
    }
    if (textFinal) {
        compacted.push(textFinal);
    }
    if (reasoningFinal) {
        compacted.push(reasoningFinal);
    }
    if (errorItem) {
        compacted.push(errorItem);
    }
    for (const item of others) {
        compacted.push(item);
    }
    return compacted;
}

export function ingestAgentHistoryItems(
    items: AiChatItem[],
    canAcceptItem: (item: AiChatItem) => boolean,
    copyBuffer: AgentTurnCopyBuffer,
    summaryBuffer: AgentRunSummaryBuffer,
    turnClock: AgentTurnClock,
): AgentHistoryIngestResult {
    const roots: AiChatItem[] = [];
    const rootIds = new Set<string>();
    const resolver = new AgentRootResolver();
    const historyById = new Map<string, AiChatItem[]>();
    const runEndByReqId = new Map<string, AiChatItem>();
    let runDone = true;

    function pushRoot(item: AiChatItem, target: AgentRootTarget): void {
        if (!rootIds.has(target.rootId)) {
            rootIds.add(target.rootId);
            roots.push(buildAgentRoot(item, target));
            return;
        }
        if (!target.self) {
            return;
        }
        for (let index = 0; index < roots.length; index++) {
            if (roots[index].id !== item.id) {
                continue;
            }
            if (roots[index].type === item.type) {
                return;
            }
            roots[index] = {...roots[index], type: item.type, context: item.context, timestamp: item.timestamp};
            return;
        }
    }

    for (const item of items) {
        if (!canAcceptItem(item)) {
            continue;
        }
        turnClock.accumulate(item);
        const nested = resolver.isNested(item);
        if (!nested) {
            copyBuffer.accumulate(item);
            summaryBuffer.accumulate(item);
        }
        if (item.type === RESP_EVENT_AGENT.ON_OPEN) {
            runDone = false;
            continue;
        }
        if (isRunEndEvent(item.type)) {
            runEndByReqId.set(item.context.reqId, item);
            runDone = true;
            continue;
        }
        const target = resolver.resolve(item);
        if (isNil(target)) {
            continue;
        }
        pushRoot(item, target);
        recordHistory(historyById, target.rootId, item);
    }

    for (const [id, history] of historyById) {
        historyById.set(id, compactHistory(history, item => resolver.isNested(item)));
    }

    return {roots, rootIds, runDone, resolver, historyById, runEndByReqId};
}

export function buildAgentRoot(item: AiChatItem, target: AgentRootTarget): AiChatItem {
    if (target.self) {
        return item;
    }
    return {
        pid: null,
        context: {},
        id: target.rootId,
        type: target.rootType,
        chat_id: item.chat_id,
        timestamp: item.timestamp,
    };
}