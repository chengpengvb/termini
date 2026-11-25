import type { RespAgentCancelled, RespAgentConflict, RespAgentConflictAnswered, RespAgentDelta, RespAgentDone, RespAgentError, RespAgentImage, RespAgentOpen, RespAgentQuestion, RespAgentShellDaemon, RespAgentShellDaemonChunk, RespAgentStatus, RespAgentToolCall, RespAgentToolCallChunk, RespAgentToolCallDone, RespAgentToolSupervise, RespAgentUser } from '@/common/model/resp/RespAgent.ts';

export const RESP_EVENT_AGENT = {
    ON_OPEN: 60,
    ON_CANCELLED: 61,
    ON_CONFLICT: 62,
    ON_DONE: 63,
    ON_ERROR: 64,
    ON_USER: 65,
    ON_TEXT: 66,
    ON_REASONING: 67,
    ON_TOOL_CALL: 68,
    ON_TOOL_CALL_DONE: 69,
    ON_TOOL_CALL_CHUNK: 70,
    ON_TEXT_FINAL: 71,
    ON_REASONING_FINAL: 72,
    ON_CONFLICT_ANSWERED: 73,
    ON_TOOL_SUPERVISE: 74,
    ON_SHELL_DAEMON: 75,
    ON_STATUS: 76,
    ON_QUESTION: 77,
    ON_SUBAGENT_TOOL_CALL: 78,
    ON_SUBAGENT_TOOL_DONE: 79,
    ON_SUBAGENT_OPEN: 80,
    ON_IMAGE: 81,
    ON_QUESTION_ANSWERED: 82,
    ON_SHELL_DAEMON_CHUNK: 83,
} as const;

const AGENT_EVENT_TYPES: ReadonlySet<number> = new Set(Object.values(RESP_EVENT_AGENT));

export function isAgentEvent(type: number): boolean {
    return AGENT_EVENT_TYPES.has(type);
}

export interface RespEventAgent {
    type: number;
    payload: RespAgentOpen | RespAgentStatus | RespAgentCancelled | RespAgentConflict | RespAgentConflictAnswered | RespAgentDone | RespAgentError | RespAgentUser | RespAgentDelta | RespAgentImage | RespAgentToolCall | RespAgentToolCallDone | RespAgentToolCallChunk | RespAgentToolSupervise | RespAgentShellDaemon | RespAgentShellDaemonChunk | RespAgentQuestion;
}
