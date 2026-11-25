import {useSyncExternalStore} from 'react';

const TICK_MS = 60_000;

let now = Date.now();
let timer = 0;
const listeners = new Set<() => void>();

function emit(): void {
    now = Date.now();
    for (const listener of listeners) {
        listener();
    }
}

function startTimer(): void {
    if (timer !== 0 || document.hidden) {
        return;
    }
    timer = window.setInterval(emit, TICK_MS);
}

function stopTimer(): void {
    if (timer === 0) {
        return;
    }
    window.clearInterval(timer);
    timer = 0;
}

function handleVisibilityChange(): void {
    if (document.hidden) {
        stopTimer();
        return;
    }
    emit();
    startTimer();
}

function subscribe(listener: () => void): () => void {
    if (listeners.size === 0) {
        document.addEventListener('visibilitychange', handleVisibilityChange);
        startTimer();
    }
    listeners.add(listener);
    return () => {
        listeners.delete(listener);
        if (listeners.size === 0) {
            document.removeEventListener('visibilitychange', handleVisibilityChange);
            stopTimer();
        }
    };
}

function getSnapshot(): number {
    return now;
}

/**
 * 全局共享的"当前时间", 每分钟广播一次; 所有订阅者共用同一个定时器,
 * 供相对时间("3 分钟前")这类分钟粒度的文案刷新, 不要用于秒级计时。
 */
export function useNowTick(): number {
    return useSyncExternalStore(subscribe, getSnapshot, getSnapshot);
}
