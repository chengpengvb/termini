import type {AgentQuestionAnswerItem} from '@/common/model/req/event/WsAgent.ts';

export interface RespAgentBase {
    reqId: string;
    sessionId: string;
    parentReqId?: string;
}

export interface RespAgentRunBase {
    runId: string;
}

export interface RespAgentOpen {
    reqId: string;
    sessionId: string;
}

export interface RespAgentStatus {
    reqId: string;
    sessionId: string;
    stage: string;
    attempt?: number;
    maxAttempts?: number;
    tokensBefore?: number;
    tokensAfter?: number;
}

export interface AgentStatusInfo {
    stage: string;
    attempt: number | null;
    maxAttempts: number | null;
}

export interface AgentCompactionInfo {
    tokensBefore: number;
    tokensAfter: number;
}

export interface RespAgentDone {
    reqId: string;
    sessionId: string;
}

export interface RespAgentCancelled {
    reqId: string;
    sessionId: string;
}

export interface RespAgentError {
    reqId: string;
    sessionId: string;
    code?: number;
    error: string;
}

export interface RespAgentUser {
    reqId: string;
    sessionId: string;
    message: string;
    origin: number;
}

export interface RespAgentDelta {
    reqId: string;
    sessionId: string;
    runId: string;
    delta: string;
}

export interface RespAgentImage {
    reqId: string;
    sessionId: string;
    runId: string;
    path: string;
    mediaType: string;
}

export interface RespAgentImageData {
    data: string;
    mediaType: string;
}

export interface RespAgentToolCall {
    reqId: string;
    sessionId: string;
    runId: string;
    callId: string;
    toolId: string;
}

export interface RespAgentToolCallDone {
    reqId: string;
    sessionId: string;
    runId: string;
    callId: string;
    result: string;
    status: number;
}

export interface RespAgentToolCallChunk {
    reqId: string;
    sessionId: string;
    runId: string;
    callId: string;
    toolId: string;
    delta: string;
}

export interface RespAgentToolSupervise {
    reqId: string;
    sessionId: string;
    runId: string;
    callId: string;
    toolId: string;
    round: number;
    action: number;
    waitSeconds: number;
    reason: string;
}

export interface RespAgentConflict {
    reqId: string;
    sessionId: string;
    runId: string;
    args: unknown;
    callId: string;
    toolId: string;
    autoAllowed: boolean;
    auditReason?: string;
    sessionPrefix?: string;
}

export interface RespAgentConflictAnswered {
    reqId: string;
    sessionId: string;
    runId: string;
    callId: string;
    toolId: string;
    code: number;
}

export interface RespAgentQuestionOption {
    id: string;
    label: string;
}

export interface RespAgentQuestionItem {
    id: string;
    prompt: string;
    options: RespAgentQuestionOption[];
    allow_multiple?: boolean;
}

export interface RespAgentQuestion {
    reqId: string;
    sessionId: string;
    runId: string;
    callId: string;
    toolId: string;
    questions: RespAgentQuestionItem[];
    state?: string;
    answers?: AgentQuestionAnswerItem[];
}

export interface RespAgentQuestionAnswered {
    callId: string;
    state: string;
    answers?: AgentQuestionAnswerItem[];
}

export interface RespAgentShellDaemonRow {
    tty: boolean;
    processId: string;
    pid: number | null;
    command: string;
    shellId: string | null;
    cwd: string | null;
    output: string;
    startedMs: number;
}

export interface RespAgentShellDaemon {
    chatId: string;
    daemons: RespAgentShellDaemonRow[];
}

export interface RespAgentShellDaemonChunk {
    chatId: string;
    processId: string;
    delta: string;
}
