export const AGENT_ANSWER_CODE = {
    ALLOW: 1,
    REJECT: 2,
    ALLOW_REMEMBER: 3,
    ALLOW_SESSION_PREFIX: 4,
} as const;

export function isAgentAnswerAllowed(code: number | null): boolean {
    return code === AGENT_ANSWER_CODE.ALLOW || code === AGENT_ANSWER_CODE.ALLOW_REMEMBER || code === AGENT_ANSWER_CODE.ALLOW_SESSION_PREFIX;
}

export interface AgentChatImage {
    mediaType: string;
    data: string;
}

export interface AgentChatData {
    message: string;
    aiChatId: string;
    aiProvideId: string;
    confirmMode: number;
    images: AgentChatImage[];
    thinkingLevel?: string | null;
}

export interface AgentCancelData {
    reqId: string;
}

export interface AgentToolAnswerData {
    code: number;
    callId: string;
}

export interface AgentToolCancelData {
    reqId: string;
    runId: string;
    callId: string;
}

export interface AgentSubscribeData {
    aiChatId: string;
}

export interface AgentConfirmModeData {
    aiChatId: string;
    confirmMode: number;
}

export interface AgentQuestionAnswerItem {
    questionId: string;
    optionIds: string[];
    text: string | null;
}

export interface AgentQuestionAnswer {
    callId: string;
    skipped: boolean;
    answers: AgentQuestionAnswerItem[];
}

export interface AgentQuestionAnswerData extends AgentQuestionAnswer {
    aiChatId: string;
    aiProvideId: string;
    confirmMode: number;
    thinkingLevel: string | null;
}

export interface AgentChatEvent {
    data: AgentChatData;
}

export interface AgentCancelEvent {
    data: AgentCancelData;
}

export interface AgentToolAnswerEvent {
    data: AgentToolAnswerData;
}

export interface AgentToolCancelEvent {
    data: AgentToolCancelData;
}

export interface AgentSubscribeEvent {
    data: AgentSubscribeData;
}

export interface AgentConfirmModeEvent {
    data: AgentConfirmModeData;
}

export interface AgentQuestionAnswerEvent {
    data: AgentQuestionAnswerData;
}
