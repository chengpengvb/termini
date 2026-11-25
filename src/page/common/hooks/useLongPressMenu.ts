import {useRef} from 'react';
import {usePlatformMobile} from '@/common/context/usePlatformMobile';
import {useSystemUtilService} from '@/common/context/useAppHooks.tsx';
import type {MouseEvent, PointerEvent as ReactPointerEvent} from 'react';
import {HAPTIC_IMPACT_STYLE, HAPTIC_KIND} from '@/common/model/dto/Haptic.ts';

const LONG_PRESS_MS = 500;
const MOVE_CANCEL_PX2 = 400;

export function useLongPressMenu(onOpen: (clientX: number, clientY: number) => void) {
    const firedRef = useRef(false);
    const mobile = usePlatformMobile();
    const timerRef = useRef<number | null>(null);
    const systemUtilService = useSystemUtilService();
    const startRef = useRef<{x: number; y: number} | null>(null);

    const clearTimer = (): void => {
        if (timerRef.current === null) {
            return;
        }
        window.clearTimeout(timerRef.current);
        timerRef.current = null;
    };

    const fireOpen = (clientX: number, clientY: number, haptic: boolean): void => {
        if (haptic && mobile && !firedRef.current) {
            void systemUtilService.haptic({kind: HAPTIC_KIND.IMPACT, style: HAPTIC_IMPACT_STYLE.MEDIUM});
        }
        firedRef.current = true;
        onOpen(clientX, clientY);
    };

    const handleContextMenu = (e: MouseEvent): void => {
        e.preventDefault();
        e.stopPropagation();
        fireOpen(e.clientX, e.clientY, true);
    };

    const handlePointerDown = (e: ReactPointerEvent): void => {
        if (e.pointerType === 'mouse' && e.button !== 0) {
            return;
        }
        firedRef.current = false;
        if (e.pointerType === 'mouse') {
            return;
        }
        startRef.current = {x: e.clientX, y: e.clientY};
        const startX = e.clientX;
        const startY = e.clientY;
        clearTimer();
        timerRef.current = window.setTimeout(() => {
            timerRef.current = null;
            fireOpen(startX, startY, true);
        }, LONG_PRESS_MS);
    };

    const handlePointerMove = (e: ReactPointerEvent): void => {
        const start = startRef.current;
        if (!start) {
            return;
        }
        const dx = e.clientX - start.x;
        const dy = e.clientY - start.y;
        if (dx * dx + dy * dy < MOVE_CANCEL_PX2) {
            return;
        }
        clearTimer();
        startRef.current = null;
    };

    const handlePointerEnd = (): void => {
        clearTimer();
        startRef.current = null;
    };

    const handlePointerLeave = (event: ReactPointerEvent): void => {
        if (event.pointerType !== 'mouse') {
            return;
        }
        handlePointerEnd();
    };

    const consumeIfFired = (): boolean => {
        if (!firedRef.current) {
            return false;
        }
        firedRef.current = false;
        return true;
    };

    return {
        handleContextMenu,
        handlePointerDown,
        handlePointerMove,
        handlePointerEnd,
        handlePointerLeave,
        consumeIfFired,
    };
}
