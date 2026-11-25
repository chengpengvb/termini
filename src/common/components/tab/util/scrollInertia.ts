const WHEEL_SCALE = 0.2;
const MAX_VELOCITY = 72;
const DECAY = 0.94;
const STOP_VELOCITY = 0.28;
const TOUCH_BOOST = 1.35;
const MIN_FLING = 0.85;

export function attachScrollInertia(el: HTMLElement): () => void {
    let raf = 0;
    let velocity = 0;
    let lastScrollLeft = el.scrollLeft;
    let lastTime = performance.now();
    let tracking = false;

    const stopInertia = (): void => {
        if (!raf) {
            return;
        }
        cancelAnimationFrame(raf);
        raf = 0;
    };

    const clampScroll = (next: number): number => {
        const max = Math.max(0, el.scrollWidth - el.clientWidth);
        if (next < 0) {
            return 0;
        }
        if (next > max) {
            return max;
        }
        return next;
    };

    const step = (): void => {
        raf = 0;
        if (Math.abs(velocity) < STOP_VELOCITY) {
            velocity = 0;
            return;
        }
        const max = Math.max(0, el.scrollWidth - el.clientWidth);
        const next = el.scrollLeft + velocity;
        if (next <= 0 || next >= max) {
            el.scrollLeft = clampScroll(next);
            velocity = 0;
            return;
        }
        el.scrollLeft = next;
        velocity *= DECAY;
        raf = requestAnimationFrame(step);
    };

    const kick = (): void => {
        if (raf) {
            return;
        }
        raf = requestAnimationFrame(step);
    };

    const addVelocity = (delta: number): void => {
        velocity += delta;
        if (velocity > MAX_VELOCITY) {
            velocity = MAX_VELOCITY;
        } else if (velocity < -MAX_VELOCITY) {
            velocity = -MAX_VELOCITY;
        }
        kick();
    };

    const sampleScrollVelocity = (): void => {
        const now = performance.now();
        const x = el.scrollLeft;
        const dt = now - lastTime;
        if (dt >= 8) {
            const nextVelocity = ((x - lastScrollLeft) * 16) / dt;
            velocity = velocity * 0.35 + nextVelocity * 0.65;
            lastScrollLeft = x;
            lastTime = now;
        }
    };

    const beginTrack = (): void => {
        if (tracking) {
            return;
        }
        tracking = true;
        stopInertia();
        velocity = 0;
        lastScrollLeft = el.scrollLeft;
        lastTime = performance.now();
    };

    const endTrack = (): void => {
        if (!tracking) {
            return;
        }
        tracking = false;
        sampleScrollVelocity();
        velocity *= TOUCH_BOOST;
        if (Math.abs(velocity) < MIN_FLING) {
            velocity = 0;
            return;
        }
        if (velocity > MAX_VELOCITY) {
            velocity = MAX_VELOCITY;
        } else if (velocity < -MAX_VELOCITY) {
            velocity = -MAX_VELOCITY;
        }
        kick();
    };

    const onWheel = (e: WheelEvent): void => {
        if (el.scrollWidth <= el.clientWidth) {
            stopInertia();
            return;
        }
        const factor = e.deltaMode === 1 ? 16 : e.deltaMode === 2 ? el.clientWidth : 1;
        const dx = e.deltaX * factor;
        const dy = e.deltaY * factor;
        if (dx !== 0) {
            stopInertia();
            return;
        }
        if (dy === 0) {
            return;
        }
        addVelocity(dy * WHEEL_SCALE);
        e.preventDefault();
    };

    const onScroll = (): void => {
        if (!tracking) {
            return;
        }
        sampleScrollVelocity();
    };

    const onPointerDown = (e: PointerEvent): void => {
        if (e.pointerType === 'mouse') {
            stopInertia();
            return;
        }
        beginTrack();
    };

    const onPointerUp = (e: PointerEvent): void => {
        if (e.pointerType === 'mouse') {
            return;
        }
        endTrack();
    };

    const onTouchStart = (): void => {
        beginTrack();
    };

    const onTouchEnd = (): void => {
        endTrack();
    };

    el.addEventListener('wheel', onWheel, {passive: false});
    el.addEventListener('scroll', onScroll, {passive: true});
    el.addEventListener('pointerdown', onPointerDown, {passive: true});
    el.addEventListener('pointerup', onPointerUp, {passive: true});
    el.addEventListener('pointercancel', onPointerUp, {passive: true});
    el.addEventListener('touchstart', onTouchStart, {passive: true});
    el.addEventListener('touchend', onTouchEnd, {passive: true});
    el.addEventListener('touchcancel', onTouchEnd, {passive: true});

    return () => {
        stopInertia();
        el.removeEventListener('wheel', onWheel);
        el.removeEventListener('scroll', onScroll);
        el.removeEventListener('pointerdown', onPointerDown);
        el.removeEventListener('pointerup', onPointerUp);
        el.removeEventListener('pointercancel', onPointerUp);
        el.removeEventListener('touchstart', onTouchStart);
        el.removeEventListener('touchend', onTouchEnd);
        el.removeEventListener('touchcancel', onTouchEnd);
    };
}
