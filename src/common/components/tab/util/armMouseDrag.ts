type ArmHandlers = {
    onPointerMove: (e: PointerEvent) => void;
    onPointerUp: (e: PointerEvent) => void;
    onPointerCancel: (e: PointerEvent) => void;
};

function disposeArmHandlers(handlers: ArmHandlers): void {
    window.removeEventListener('pointerup', handlers.onPointerUp);
    window.removeEventListener('pointermove', handlers.onPointerMove);
    window.removeEventListener('pointercancel', handlers.onPointerCancel);
}

function distSq(ax: number, ay: number, bx: number, by: number): number {
    const dx = bx - ax;
    const dy = by - ay;
    return dx * dx + dy * dy;
}

export function armPointerDrag(pointerId: number, startX: number, startY: number, dragStartDistancePx: number, onStart: (clientX: number, clientY: number) => void, onMove: (clientX: number, clientY: number) => void, onEnd: (clientX: number, clientY: number) => void): () => void {
    const ctx = {started: false};
    const thresholdSq = dragStartDistancePx * dragStartDistancePx;
    const handlers = createArmHandlers(pointerId, startX, startY, thresholdSq, ctx, onStart, onMove, onEnd);
    window.addEventListener('pointerup', handlers.onPointerUp);
    window.addEventListener('pointermove', handlers.onPointerMove);
    window.addEventListener('pointercancel', handlers.onPointerCancel);
    return () => disposeArmHandlers(handlers);
}

function createArmHandlers(pointerId: number, startX: number, startY: number, thresholdSq: number, ctx: { started: boolean }, onStart: (clientX: number, clientY: number) => void, onMove: (clientX: number, clientY: number) => void, onEnd: (clientX: number, clientY: number) => void): ArmHandlers {
    function onPointerMove(e: PointerEvent): void {
        if (e.pointerId !== pointerId) {
            return;
        }
        if (!ctx.started) {
            if (distSq(startX, startY, e.clientX, e.clientY) < thresholdSq) {
                return;
            }
            ctx.started = true;
            onStart(e.clientX, e.clientY);
            return;
        }
        onMove(e.clientX, e.clientY);
    }

    function onPointerUp(e: PointerEvent): void {
        if (e.pointerId !== pointerId) {
            return;
        }
        disposeArmHandlers({onPointerMove, onPointerUp, onPointerCancel});
        if (!ctx.started) {
            return;
        }
        onEnd(e.clientX, e.clientY);
    }

    function onPointerCancel(e: PointerEvent): void {
        if (e.pointerId !== pointerId) {
            return;
        }
        disposeArmHandlers({onPointerMove, onPointerUp, onPointerCancel});
        if (!ctx.started) {
            return;
        }
        onEnd(e.clientX, e.clientY);
    }

    return {onPointerMove, onPointerUp, onPointerCancel};
}

