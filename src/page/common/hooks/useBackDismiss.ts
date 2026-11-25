import {useEffect, useRef} from 'react';
import {pushBackDismiss} from '@/page/common/util/backDismissRegistry.ts';

export function useBackDismiss(active: boolean, dismiss: () => void, id?: string): void {
    const dismissRef = useRef(dismiss);
    dismissRef.current = dismiss;

    useEffect(() => {
        if (!active) {
            return;
        }
        return pushBackDismiss(() => {
            dismissRef.current();
        }, id);
    }, [active, id]);
}
