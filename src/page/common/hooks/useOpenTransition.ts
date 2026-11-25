import {useEffect, useState} from 'react';

export function useOpenTransition(open: boolean, durationMs = 340): {mounted: boolean; active: boolean} {
    const [active, setActive] = useState(false);
    const [mounted, setMounted] = useState(open);

    useEffect(() => {
        if (open) {
            setMounted(true);
            const timer = window.setTimeout(() => {
                setActive(true);
            }, 32);
            return () => {
                window.clearTimeout(timer);
            };
        }
        setActive(false);
        const timer = window.setTimeout(() => {
            setMounted(false);
        }, durationMs);
        return () => {
            window.clearTimeout(timer);
        };
    }, [open, durationMs]);

    return {mounted, active};
}
