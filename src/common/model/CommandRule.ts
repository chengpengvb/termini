export const RULE_DECISION = {
    ALLOW: 1,
    PROMPT: 2,
    FORBIDDEN: 3,
} as const;

export interface CommandRule {
    id: string;
    pattern: string;
    decision: number;
    device_id: string;
    created_at: number;
    justification: string;
}

export interface CreateCommandRuleReq {
    pattern: string;
    decision: number;
    deviceId: string | null;
    justification: string | null;
}

export function formatRulePattern(rule: CommandRule): string {
    const parsed: unknown = JSON.parse(rule.pattern);
    if (!Array.isArray(parsed)) {
        return rule.pattern;
    }
    return parsed.map(item => (Array.isArray(item) ? item.join('|') : String(item))).join(' ');
}
