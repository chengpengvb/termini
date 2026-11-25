import {isNil} from '@/common/util/StringUtil.ts';
import type {AiChatItem} from '@/common/model/AiChatItem.ts';
import {RESP_EVENT_AGENT} from '@/common/model/resp/event/RespEventAgent.ts';

export const TURN_END_STATE = {
    DONE: 0,
    CANCELLED: 1,
    FAILED: 2,
} as const;

export interface AgentTurnTiming {
    startAt: number;
    endAt: number;
    state: number;
    approximate: boolean;
}

interface TurnClockEntry {
    startAt: number | null;
    lastAt: number;
    endAt: number | null;
    cancelled: boolean;
    failed: boolean;
}

export class AgentTurnClock {
    private readonly entries = new Map<string, TurnClockEntry>();

    clear(): void {
        this.entries.clear();
    }

    accumulate(item: AiChatItem): boolean {
        if (item.context?.parentReqId) {
            return false;
        }
        const reqId: string | undefined = item.context?.reqId;
        if (!reqId) {
            return false;
        }
        const entry = this.resolveEntry(reqId);
        entry.lastAt = Math.max(entry.lastAt, item.timestamp);
        if (item.type === RESP_EVENT_AGENT.ON_USER) {
            entry.startAt = item.timestamp;
            return true;
        }
        if (item.type === RESP_EVENT_AGENT.ON_ERROR) {
            entry.failed = true;
            return true;
        }
        if (item.type === RESP_EVENT_AGENT.ON_CANCELLED) {
            entry.cancelled = true;
            if (isNil(entry.endAt)) {
                entry.endAt = item.timestamp;
            }
            return true;
        }
        if (item.type === RESP_EVENT_AGENT.ON_DONE) {
            if (isNil(entry.endAt)) {
                entry.endAt = item.timestamp;
            }
            return true;
        }
        return false;
    }

    resolve(reqId: string): AgentTurnTiming | null {
        const entry = this.entries.get(reqId);
        if (!entry || isNil(entry.startAt)) {
            return null;
        }
        const endAt = entry.endAt ?? entry.lastAt;
        if (endAt < entry.startAt) {
            return null;
        }
        return {
            startAt: entry.startAt,
            endAt,
            state: this.resolveState(entry),
            approximate: isNil(entry.endAt),
        };
    }

    private resolveState(entry: TurnClockEntry): number {
        if (entry.cancelled) {
            return TURN_END_STATE.CANCELLED;
        }
        if (entry.failed) {
            return TURN_END_STATE.FAILED;
        }
        return TURN_END_STATE.DONE;
    }

    private resolveEntry(reqId: string): TurnClockEntry {
        const existing = this.entries.get(reqId);
        if (existing) {
            return existing;
        }
        const created: TurnClockEntry = {startAt: null, lastAt: 0, endAt: null, cancelled: false, failed: false};
        this.entries.set(reqId, created);
        return created;
    }
}
