export interface ContextUsage {
    total: number;
    live: boolean;
    measured: number;
    summary: number;
    mcpTools: number;
    tokenLimit: number;
    conversation: number;
    systemPrompt: number;
    toolDefinitions: number;
}
