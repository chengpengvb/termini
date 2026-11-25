export const DEFAULT_CONFIG_ID = '000000';

export const SUGGEST_COUNT_MIN = 1;
export const SUGGEST_COUNT_MAX = 10;
export const SUGGEST_COUNT_DEFAULT = 3;

export const SYSTEM_LANGUAGE = {
    ZH_CN: 0,
    EN_US: 1,
    JA_JP: 2,
    KO_KR: 3,
    DE_DE: 4,
    FR_FR: 5,
    ES_ES: 6,
    IT_IT: 7,
    PT_PT: 8,
    NL_NL: 9,
    ZH_TW: 10,
    HI_IN: 11,
} as const;

export const SYSTEM_LANGUAGE_OPTIONS = [
    SYSTEM_LANGUAGE.ZH_CN,
    SYSTEM_LANGUAGE.EN_US,
    SYSTEM_LANGUAGE.JA_JP,
    SYSTEM_LANGUAGE.KO_KR,
    SYSTEM_LANGUAGE.DE_DE,
    SYSTEM_LANGUAGE.FR_FR,
    SYSTEM_LANGUAGE.ES_ES,
    SYSTEM_LANGUAGE.IT_IT,
    SYSTEM_LANGUAGE.PT_PT,
    SYSTEM_LANGUAGE.NL_NL,
    SYSTEM_LANGUAGE.ZH_TW,
    SYSTEM_LANGUAGE.HI_IN,
] as const;

export const SHELL_CPU_PERCENT_MIN = 10;
export const SHELL_CPU_PERCENT_MAX = 100;
export const SHELL_CPU_PERCENT_DEFAULT = 80;

export const CLOSE_ACTION = {
    ASK: 0,
    EXIT: 1,
    TRAY: 2,
} as const;

export const WEBHOOK_KIND = {
    DINGTALK: 1,
    WECOM: 2,
    FEISHU: 3,
    TELEGRAM: 4,
} as const;

export const MODEL_ROLE = {
    VISION: 0,
    IMAGE_GEN: 1,
    EMBEDDING: 2,
    ASSIST: 3,
} as const;

export type ModelRole = typeof MODEL_ROLE[keyof typeof MODEL_ROLE];

export const JOB_EXEC_TIMEOUT_SECS_MIN = 5;
export const JOB_EXEC_TIMEOUT_SECS_MAX = 300;
export const JOB_EXEC_TIMEOUT_SECS_DEFAULT = 30;

export const JOB_WORKER_PARALLELISM_MIN = 1;
export const JOB_WORKER_PARALLELISM_MAX = 200;
export const JOB_WORKER_PARALLELISM_DEFAULT = 25;

export const JOB_SSH_POOL_MAX_SIZE_MIN = 1;
export const JOB_SSH_POOL_MAX_SIZE_MAX = 100;
export const JOB_SSH_POOL_MAX_SIZE_DEFAULT = 15;

export const JOB_SSH_POOL_IDLE_SECS_MIN = 5;
export const JOB_SSH_POOL_IDLE_SECS_MAX = 600;
export const JOB_SSH_POOL_IDLE_SECS_DEFAULT = 15;

export const JOB_DISCOVER_PARALLELISM_MIN = 1;
export const JOB_DISCOVER_PARALLELISM_MAX = 10;
export const JOB_DISCOVER_PARALLELISM_DEFAULT = 2;

export const JOB_INDICATOR_VALUE_CACHE_SIZE_MIN = 12;
export const JOB_INDICATOR_VALUE_CACHE_SIZE_MAX = 500;
export const JOB_INDICATOR_VALUE_CACHE_SIZE_DEFAULT = 36;

export interface Config {
    id?: string;
    local_shell_id?: string;
    open_here_enabled?: boolean;
    suggest_enabled?: boolean;
    suggest_count?: number;
    suggest_ai_provide_id?: string;
    index_ignore?: string | null;
    index_dirs?: string | null;
    index_include_tests?: boolean;
    system_language?: number;
    shell_cpu_percent?: number;
    job_exec_timeout_secs?: number;
    job_worker_parallelism?: number;
    job_ssh_pool_max_size?: number;
    job_ssh_pool_idle_secs?: number;
    job_discover_parallelism?: number;
    job_indicator_value_cache_size?: number;
    job_alert_ai_provide_id?: string;
    job_discover_ai_provide_id?: string;
    vision_use_cloud?: boolean;
    vision_local_ai_provide_id?: string | null;
    image_gen_use_cloud?: boolean;
    image_gen_local_ai_provide_id?: string | null;
    embedding_use_cloud?: boolean;
    embedding_local_ai_provide_id?: string | null;
    embedding_local_dim?: number | null;
    assist_use_cloud?: boolean;
    assist_local_ai_provide_id?: string | null;
    notify_email?: string;
    notify_enabled?: boolean;
    notify_tray_enabled?: boolean;
    notify_webhook_enabled?: boolean;
    notify_webhook_kind?: number;
    notify_webhook_url?: string;
    notify_webhook_extra?: string;
    close_action?: number;
    web_fetch_trusted_hosts?: string | null;
    audit_policy?: string | null;
    confirm_prefix_skip_tokens?: string | null;
    confirm_prefix_tools?: string | null;
}

/* eslint-disable termini/no-cjk-literal -- 语言名按其自身语言书写 */
const SYSTEM_LANGUAGE_LABEL: Record<number, string> = {
    [SYSTEM_LANGUAGE.ZH_CN]: '简体中文',
    [SYSTEM_LANGUAGE.EN_US]: 'English',
    [SYSTEM_LANGUAGE.JA_JP]: '日本語',
    [SYSTEM_LANGUAGE.KO_KR]: '한국어',
    [SYSTEM_LANGUAGE.DE_DE]: 'Deutsch',
    [SYSTEM_LANGUAGE.FR_FR]: 'Français',
    [SYSTEM_LANGUAGE.ES_ES]: 'Español',
    [SYSTEM_LANGUAGE.IT_IT]: 'Italiano',
    [SYSTEM_LANGUAGE.PT_PT]: 'Português',
    [SYSTEM_LANGUAGE.NL_NL]: 'Nederlands',
    [SYSTEM_LANGUAGE.ZH_TW]: '繁體中文',
    [SYSTEM_LANGUAGE.HI_IN]: 'हिंदी',
};
/* eslint-enable termini/no-cjk-literal */

export function systemLanguageLabel(language: number): string {
    const label = SYSTEM_LANGUAGE_LABEL[language];
    if (label == null) {
        throw new Error(`unknown system language: ${language}`);
    }
    return label;
}

export function resolveSystemLanguageFromLocale(tag: string): number {
    const normalized = tag.trim().replace(/_/g, '-').toLowerCase();
    const primary = normalized.split('.')[0] ?? '';
    if (primary.startsWith('zh-hant') || primary.startsWith('zh-tw') || primary.startsWith('zh-hk') || primary.startsWith('zh-mo')) {
        return SYSTEM_LANGUAGE.ZH_TW;
    }
    if (primary.startsWith('zh')) {
        return SYSTEM_LANGUAGE.ZH_CN;
    }
    const lang = primary.split('-')[0] ?? '';
    if (lang === 'ja') {
        return SYSTEM_LANGUAGE.JA_JP;
    }
    if (lang === 'ko') {
        return SYSTEM_LANGUAGE.KO_KR;
    }
    if (lang === 'de') {
        return SYSTEM_LANGUAGE.DE_DE;
    }
    if (lang === 'fr') {
        return SYSTEM_LANGUAGE.FR_FR;
    }
    if (lang === 'es') {
        return SYSTEM_LANGUAGE.ES_ES;
    }
    if (lang === 'it') {
        return SYSTEM_LANGUAGE.IT_IT;
    }
    if (lang === 'pt') {
        return SYSTEM_LANGUAGE.PT_PT;
    }
    if (lang === 'nl') {
        return SYSTEM_LANGUAGE.NL_NL;
    }
    if (lang === 'hi') {
        return SYSTEM_LANGUAGE.HI_IN;
    }
    return SYSTEM_LANGUAGE.EN_US;
}
