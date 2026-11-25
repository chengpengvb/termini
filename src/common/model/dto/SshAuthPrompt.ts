export const SSH_AUTH_METHOD_CODE = {
    NONE: 0,
    PASSWORD: 1,
    PUBLIC_KEY: 2,
    KEYBOARD_INTERACTIVE: 3,
} as const;

export const SSH_FINGERPRINT_REASON = {
    FIRST_SEEN: 1,
    CHANGED: 2,
} as const;

export interface InputPrompt {
    request_id: string;
}

export interface RetryPrompt {
    request_id: string;
    last_error: string;
}

export interface FingerprintPrompt {
    port: number;
    reason: number;
    host: string;
    sha256: string;
    request_id: string;
    algorithm: string;
    previous_sha256: string | null;
}

export interface InteractiveQuestion {
    text: string;
    echo: boolean;
}

export interface InteractivePrompt {
    request_id: string;
    name: string;
    instruction: string;
    questions: InteractiveQuestion[];
}

export interface MethodSelectPrompt {
    request_id: string;
    methods: number[];
    default: number | null;
}

export interface AgentKeyEntry {
    key_id: string;
    comment: string;
    algorithm: string;
    fingerprint: string;
}

export interface AgentKeyPrompt {
    request_id: string;
    agent_type: string;
    keys: AgentKeyEntry[];
}
