import {isNotEmpty, isNotNil} from '@/common/util/StringUtil.ts';
import type {RespProvider} from '@/common/model/resp/RespProvider.ts';

export const AI_PROVIDE_SOURCE = {CUSTOM: 0, CLOUD: 1} as const;

export const AI_PROVIDE_KIND = {CHAT: 0, EMBEDDING: 1, IMAGE_GEN: 2, VIDEO: 3} as const;

export interface AiProvide {
    id?: string;
    source: number;
    kind?: number;
    hidden?: boolean;
    vision?: boolean;
    url: string | null;
    key: string | null;
    model: string | null;
    provider: number | null;
    max_token: number | null;
    input_billing_rate?: number | null;
    output_billing_rate?: number | null;
    params?: Record<string, unknown> | null;
    thinking_levels?: ThinkingLevels | null;
}

export interface ThinkingLevels {
    default?: string | null;
    levels: Record<string, Record<string, unknown>>;
}

export interface SelectedProvide {
    id: string;
    source: number;
}

const THINKING_LEVEL_ORDER = ['low', 'high', 'max'];

export function resolveThinkingLevelNames(provide: AiProvide | null): string[] {
    if (!provide || !provide.thinking_levels) {
        return [];
    }
    const rank = (name: string): number => {
        const index = THINKING_LEVEL_ORDER.indexOf(name);
        return index === -1 ? THINKING_LEVEL_ORDER.length : index;
    };
    return Object.keys(provide.thinking_levels.levels).sort((a, b) => rank(a) - rank(b) || a.localeCompare(b));
}

export function resolveDefaultThinkingLevel(provide: AiProvide | null): string | null {
    const names = resolveThinkingLevelNames(provide);
    if (names.length === 0) {
        return null;
    }
    const preferred = provide?.thinking_levels?.default;
    if (isNotEmpty(preferred) && names.includes(preferred)) {
        return preferred;
    }
    return names[0];
}

export function resolveAiProvideProviderName(provide: AiProvide, templateList: RespProvider[]): string {
    if (isNotNil(provide.provider)) {
        const found = templateList.find(item => item.provide_id === provide.provider);
        if (isNotNil(found)) {
            return found.name;
        }
    }
    return 'Custom';
}

export function resolveAiProvideModelLabel(provide: AiProvide, templateList: RespProvider[]): string {
    if (isNotEmpty(provide.model)) {
        return provide.model;
    }
    return resolveAiProvideProviderName(provide, templateList);
}

function formatAiProvideRate(value: number): string {
    return String(Number(value.toPrecision(3)));
}

export function resolveAiProvideRateLabel(provide: AiProvide): string | null {
    const input = provide.input_billing_rate;
    const output = provide.output_billing_rate;
    if (!isNotNil(input) || !isNotNil(output)) {
        return null;
    }
    const inputText = formatAiProvideRate(input);
    const outputText = formatAiProvideRate(output);
    if (input === output) {
        return `×${inputText}`;
    }
    return `×${inputText}/${outputText}`;
}
