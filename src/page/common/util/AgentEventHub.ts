import {isNil, isNotNil} from '@/common/util/StringUtil.ts';
import type {AiChatItem} from '@/common/model/AiChatItem.ts';
import type {ItemHandle} from '@/page/common/dto/agent/ItemHandle.ts';
import {AgentRootResolver, type AgentHistoryIngestResult, type AgentRootTarget} from '@/page/common/util/agentMessageHistory.ts';

export class AgentEventHub {
    private rootIds = new Set<string>();
    private resolver = new AgentRootResolver();
    private historyById = new Map<string, AiChatItem[]>();
    private runEndByReqId = new Map<string, AiChatItem>();
    private readonly handleById = new Map<string, ItemHandle>();

    register(id: string, handle: ItemHandle | null): void {
        if (isNil(handle)) {
            this.handleById.delete(id);
            return;
        }
        this.handleById.set(id, handle);
        handle.replay(this.replayItems(id));
    }

    resolve(item: AiChatItem): AgentRootTarget | null {
        return this.resolver.resolve(item);
    }

    isNested(item: AiChatItem): boolean {
        return this.resolver.isNested(item);
    }

    nestedRootId(item: AiChatItem): string | null {
        return this.resolver.nestedRootId(item);
    }

    hasRoot(rootId: string): boolean {
        return this.rootIds.has(rootId);
    }

    addRoot(rootId: string): void {
        this.rootIds.add(rootId);
    }

    addRunEnd(reqId: string, item: AiChatItem): void {
        this.runEndByReqId.set(reqId, item);
    }

    dispatch(rootId: string, item: AiChatItem): void {
        const history = this.historyById.get(rootId);
        if (isNil(history)) {
            this.historyById.set(rootId, [item]);
        } else {
            history.push(item);
        }
        const handle = this.handleById.get(rootId);
        if (isNotNil(handle)) {
            handle.onEvent(item);
        }
    }

    broadcast(item: AiChatItem): void {
        for (const handle of this.handleById.values()) {
            handle.onEvent(item);
        }
    }

    applyIngest(ingested: AgentHistoryIngestResult): void {
        this.rootIds = ingested.rootIds;
        this.resolver = ingested.resolver;
        this.historyById = ingested.historyById;
        this.runEndByReqId = ingested.runEndByReqId;
    }

    clear(): void {
        this.rootIds.clear();
        this.resolver.clear();
        this.handleById.clear();
        this.historyById.clear();
        this.runEndByReqId.clear();
    }

    private replayItems(id: string): AiChatItem[] {
        const history = this.historyById.get(id);
        if (isNil(history)) {
            return [];
        }
        const items = [...history];
        const owner = history.find(item => isNotNil(item.context?.reqId));
        if (isNil(owner)) {
            return items;
        }
        const runEnd = this.runEndByReqId.get(owner.context.reqId);
        if (isNotNil(runEnd)) {
            items.push(runEnd);
        }
        return items;
    }
}
