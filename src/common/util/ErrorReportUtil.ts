import {invoke} from '@tauri-apps/api/core';
import {API_BASE_URL} from '@/runtime-config.ts';
import {inTauri} from '@/common/util/TauriUtil.ts';
import {RespRunMarker} from '@/common/model/resp/RespRunMarker.ts';

const STACK_MAX_CHARS = 4096;
const MESSAGE_MAX_CHARS = 1024;
const REPORT_MAX_PER_SESSION = 10;

let reportedCount = 0;
let lastMessage = '';

export function reportError(source: string, message: string, stack: string | null): void {
    if (reportedCount >= REPORT_MAX_PER_SESSION) {
        return;
    }
    const trimmed = message.slice(0, MESSAGE_MAX_CHARS);
    if (trimmed === lastMessage) {
        return;
    }
    lastMessage = trimmed;
    reportedCount += 1;
    void fetch(`${API_BASE_URL}/account/report_error`, {
        method: 'POST',
        headers: {'Content-Type': 'application/json'},
        body: JSON.stringify({
            source,
            message: trimmed,
            stack: stack === null ? null : stack.slice(0, STACK_MAX_CHARS),
        }),
    }).catch(() => {
        reportedCount = REPORT_MAX_PER_SESSION;
    });
}

const CRASH_REPORT_DELAY_MS = 15_000;

export function installCrashReport(): void {
    if (!inTauri()) {
        return;
    }
    window.setTimeout(() => {
        invoke<RespRunMarker | null>('take_last_crash').then((marker) => {
            if (marker === null) {
                return;
            }
            const startedAt = new Date(marker.startedAt).toISOString();
            reportError('crash', `previous run exited abnormally: pid=${marker.pid} version=${marker.version} startedAt=${startedAt}`, null);
        }).catch((error: unknown) => {
            console.error('[crash] take_last_crash failed', error);
        });
    }, CRASH_REPORT_DELAY_MS);
}

export function installErrorReport(): void {
    window.addEventListener('error', (event) => {
        const error = event.error as Error | undefined;
        reportError('web', event.message, error?.stack ?? null);
    });
    window.addEventListener('unhandledrejection', (event) => {
        const reason: unknown = event.reason;
        if (reason instanceof Error) {
            reportError('web', reason.message, reason.stack ?? null);
            return;
        }
        reportError('web', String(reason), null);
    });
}
