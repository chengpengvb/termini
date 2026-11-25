export interface AiMcpServer {
    sort: number;
    id: string | null;
    enabled: boolean;
    transport: number;
    name: string | null;
    args: string | null;
    env: string | null;
    url: string | null;
    command: string | null;
    headers: string | null;
    timeout_secs: number | null;
}

export interface McpToolSnapshot {
    name: string;
    schema: unknown;
    server_id: string;
    description: string;
    server_name: string;
}

export interface RespAiMcpServerRuntime {
    server: AiMcpServer;
    connected: boolean;
    tool_count: number;
    last_error: string | null;
}

export interface RespAiMcpServerTools {
    tools: McpToolSnapshot[];
}

export interface McpPromptArgument {
    name: string;
    required: boolean;
    description: string;
}

export interface McpPromptSnapshot {
    name: string;
    server_id: string;
    description: string;
    server_name: string;
    arguments: McpPromptArgument[];
}

export const MCP_MARKET_RUNTIME = {
    NODE: 0,
    PYTHON: 1,
    BINARY: 2,
    HTTP: 3,
} as const;

export interface McpMarketItem {
    id: string;
    sort: number;
    enabled: boolean;
    name: string;
    title: string | null;
    description: string | null;
    icon: string | null;
    category: string | null;
    runtime: number;
    pkg: string | null;
    args: string | null;
    envKeys: string | null;
    url: string | null;
    homepage: string | null;
    updateTime: number;
}

export interface McpMarketEnvKey {
    key: string;
    label: string | null;
    required: boolean;
    placeholder: string | null;
}

export interface ReqMcpMarketInstall {
    marketId: string;
    env: Record<string, string>;
}
