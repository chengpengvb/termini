import {useEffect} from 'react';

let holders = 0;
let requesting = false;
let listening = false;
let sentinel: WakeLockSentinel | null = null;

function isSupported(): boolean {
    return 'wakeLock' in navigator;
}

function releaseSentinel(target: WakeLockSentinel): void {
    target.release().catch(error => {
        console.error('[keep-awake] release error', error);
    });
}

function releaseLock(): void {
    const current = sentinel;
    if (!current) {
        return;
    }
    sentinel = null;
    releaseSentinel(current);
}

function acquireLock(): void {
    if (requesting || sentinel !== null || document.visibilityState !== 'visible') {
        return;
    }
    requesting = true;
    navigator.wakeLock
        .request('screen')
        .then(next => {
            requesting = false;
            if (holders === 0) {
                releaseSentinel(next);
                return;
            }
            sentinel = next;
            next.addEventListener('release', () => {
                if (sentinel === next) {
                    sentinel = null;
                }
            });
        })
        .catch(error => {
            requesting = false;
            console.error('[keep-awake] request error', error);
        });
}

function syncLock(): void {
    if (holders === 0) {
        releaseLock();
        return;
    }
    acquireLock();
}

function pushKeepAwake(): () => void {
    if (!listening) {
        listening = true;
        document.addEventListener('visibilitychange', syncLock);
    }
    holders += 1;
    syncLock();
    let released = false;
    return () => {
        if (released) {
            return;
        }
        released = true;
        holders -= 1;
        syncLock();
    };
}

export function useKeepAwake(active: boolean): void {
    useEffect(() => {
        if (!active || !isSupported()) {
            return;
        }
        return pushKeepAwake();
    }, [active]);
}
