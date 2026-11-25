export const CHAT_TARGET_TYPE = {
    DEVICE: 0,
    GROUP: 1,
    LOCAL_PATH: 2,
    SKILL: 3,
    MCP_PROMPT: 4,
    MCP_TOOL: 5,
    MCP_SERVER: 6,
} as const;

export const CHAT_TARGET_KEY = {
    DEVICE: 'device',
    GROUP: 'group',
    LOCAL_PATH: 'localpath',
    SKILL: 'skill',
    MCP_PROMPT: 'mcpprompt',
    MCP_TOOL: 'mcptool',
    MCP_SERVER: 'mcpserver',
} as const;
