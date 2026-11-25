export interface ReplaceConflictView {
    path: string;
    displayName: string;
    old_string: string;
    new_string: string;
    fullPath: string | null;
    startLine: number | null;
}

export interface ReplaceFileGroup {
    key: string;
    path: string;
    displayName: string;
    fullPath: string | null;
    edits: ReplaceConflictView[];
}

export interface SearchReplaceToolTitles {
    skipped: string;
    completedFallback: string;
    running: string;
    waitingConfirm: string;
    preparing: string;
}

export interface FileReadToolTitles {
    done: string;
    reading: string;
    detached?: string;
    failed?: string;
    cancelled?: string;
    batchDone?: (count: number) => string;
    batchReading?: (count: number) => string;
}

export interface ShellConflictView {
    command: string;
    detail: string | null;
}

export interface ShellToolTitles {
    skipped: string;
    completed: string;
    running: string;
    waitingConfirm: string;
    preparing: string;
}

export interface LegacyStep {
    args: string;
    ok?: boolean;
    toolId: string;
    subCallId: string;
    elapsedMs?: number;
}

export interface SuperviseRound {
    round: number;
    action: number;
    reason: string;
    waitSeconds: number;
}

export const SUPERVISE_ACTION = {
    WAIT: 1,
    KILL: 2,
    DETACH: 3,
} as const;

export const CONTEXT_GROUP_VARIANT = {
    QUERY: 'query',
    WRITE: 'write',
} as const;

export type ContextGroupVariant = (typeof CONTEXT_GROUP_VARIANT)[keyof typeof CONTEXT_GROUP_VARIANT];
