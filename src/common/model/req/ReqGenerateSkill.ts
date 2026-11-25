export interface ReqGenerateSkill {
    taskId: string;
    aiProvideId: string;
    name?: string | null;
    chatId: string | null;
    content?: string | null;
    keyPoints?: string | null;
    description?: string | null;
}

export interface ReqCancelGenerateSkill {
    taskId: string;
}
