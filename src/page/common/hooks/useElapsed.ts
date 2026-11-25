import {useEffect, useState} from 'react';
import {isNil} from '@/common/util/StringUtil.ts';

export function formatElapsed(ms: number): string {
    const total = Math.max(0, Math.floor(ms / 1000));
    const hours = Math.floor(total / 3600);
    const minutes = Math.floor((total % 3600) / 60);
    const seconds = total % 60;
    if (hours > 0) {
        return `${hours}h${String(minutes).padStart(2, '0')}m`;
    }
    if (minutes > 0) {
        return `${minutes}m${String(seconds).padStart(2, '0')}s`;
    }
    return `${seconds}s`;
}

export function useElapsedMs(startedAt: number | null, active: boolean): number | null {
    const [now, setNow] = useState(() => Date.now());

    useEffect(() => {
        if (!active || isNil(startedAt)) {
            return;
        }
        setNow(Date.now());
        const timer = window.setInterval(() => {
            setNow(Date.now());
        }, 1000);
        return () => {
            window.clearInterval(timer);
        };
    }, [active, startedAt]);

    if (isNil(startedAt)) {
        return null;
    }
    return now - startedAt;
}

export function useElapsed(startedAt: number | null, active: boolean): string | null {
    const ms = useElapsedMs(startedAt, active);
    return isNil(ms) ? null : formatElapsed(ms);
}
