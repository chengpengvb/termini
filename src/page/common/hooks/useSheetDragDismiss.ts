import {useRef, useState, type CSSProperties, type PointerEvent as ReactPointerEvent} from 'react';

const AXIS_LOCK_PX = 6;
const EDGE_ZONE_PX = 24;
const AXIS_VERTICAL = 0;
const AXIS_HORIZONTAL = 1;
const DISMISS_DISTANCE = 96;
const DISMISS_VELOCITY = 0.85;

interface SheetDragDismissBind {
    onPointerDown: (event: ReactPointerEvent<HTMLElement>) => void;
    onPointerMove: (event: ReactPointerEvent<HTMLElement>) => void;
    onPointerUp: (event: ReactPointerEvent<HTMLElement>) => void;
    onPointerCancel: (event: ReactPointerEvent<HTMLElement>) => void;
}

export function useSheetDragDismiss(onClose: () => void, enabled: boolean): {
    panelStyle: CSSProperties;
    bindPanel: SheetDragDismissBind;
    bindHandle: SheetDragDismissBind;
} {
    const dragYRef = useRef(0);
    const capturedRef = useRef(false);
    const fromHandleRef = useRef(false);
    const [dragY, setDragYState] = useState(0);
    const axisRef = useRef<number | null>(null);
    const startRef = useRef<{x: number; y: number; t: number} | null>(null);

    const setDragY = (next: number): void => {
        dragYRef.current = next;
        setDragYState(next);
    };

    const reset = (): void => {
        startRef.current = null;
        axisRef.current = null;
        fromHandleRef.current = false;
        capturedRef.current = false;
        setDragY(0);
    };

    const begin = (event: ReactPointerEvent<HTMLElement>, fromHandle: boolean): void => {
        if (!enabled || (event.pointerType === 'mouse' && event.button !== 0)) {
            return;
        }
        if (!fromHandle && event.clientX < EDGE_ZONE_PX) {
            return;
        }
        fromHandleRef.current = fromHandle;
        startRef.current = {x: event.clientX, y: event.clientY, t: performance.now()};
        axisRef.current = fromHandle ? AXIS_VERTICAL : null;
    };

    const move = (event: ReactPointerEvent<HTMLElement>): void => {
        const start = startRef.current;
        if (!start || !enabled) {
            return;
        }
        const dy = event.clientY - start.y;
        const dx = event.clientX - start.x;
        if (axisRef.current === null) {
            if (Math.abs(dy) < AXIS_LOCK_PX && Math.abs(dx) < AXIS_LOCK_PX) {
                return;
            }
            axisRef.current = Math.abs(dy) >= Math.abs(dx) ? AXIS_VERTICAL : AXIS_HORIZONTAL;
            if (axisRef.current !== AXIS_VERTICAL) {
                return;
            }
        }
        if (axisRef.current !== AXIS_VERTICAL) {
            return;
        }
        if (!fromHandleRef.current && event.currentTarget.scrollTop > 0 && dy < 0) {
            return;
        }
        if (!capturedRef.current) {
            capturedRef.current = true;
            try {
                event.currentTarget.setPointerCapture(event.pointerId);
            } catch {
            }
        }
        setDragY(Math.max(0, dy));
    };

    const end = (): void => {
        const start = startRef.current;
        if (!start || !enabled) {
            reset();
            return;
        }
        if (axisRef.current !== AXIS_VERTICAL) {
            reset();
            return;
        }
        const elapsed = Math.max(1, performance.now() - start.t);
        const velocity = dragYRef.current / elapsed;
        const shouldClose = dragYRef.current >= DISMISS_DISTANCE || velocity >= DISMISS_VELOCITY;
        if (shouldClose) {
            onClose();
        }
        reset();
    };

    const bind = (fromHandle: boolean): SheetDragDismissBind => ({
        onPointerDown: (event) => begin(event, fromHandle),
        onPointerMove: move,
        onPointerUp: end,
        onPointerCancel: end,
    });

    const panelStyle: CSSProperties = dragY > 0
        ? {
            transition: 'none',
            transform: `translateY(${dragY}px)`,
        }
        : {};

    return {
        panelStyle,
        bindPanel: bind(false),
        bindHandle: bind(true),
    };
}
