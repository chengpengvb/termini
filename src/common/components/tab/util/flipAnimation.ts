const FLIP_EPS_SCALE = 0.01;
const FLIP_EPS_TRANSLATE = 1;
const FLIPPING_CLASS = 'tm-flipping';

export function snapshotElementRects(elements: Map<string, HTMLElement>): Map<string, DOMRect> {
    const map = new Map<string, DOMRect>();
    elements.forEach((el, id) => {
        map.set(id, el.getBoundingClientRect());
    });
    return map;
}

export function applyFlipInverse(elements: Map<string, HTMLElement>, oldRects: Map<string, DOMRect>): HTMLElement[] {
    const targets: HTMLElement[] = [];
    elements.forEach((el, id) => {
        const oldRect = oldRects.get(id);
        if (!oldRect) {
            return;
        }
        const newRect = el.getBoundingClientRect();
        if (newRect.width === 0 || newRect.height === 0) {
            return;
        }
        const dx = oldRect.left - newRect.left;
        const dy = oldRect.top - newRect.top;
        const sx = oldRect.width / newRect.width;
        const sy = oldRect.height / newRect.height;
        if (Math.abs(dx) < FLIP_EPS_TRANSLATE && Math.abs(dy) < FLIP_EPS_TRANSLATE && Math.abs(sx - 1) < FLIP_EPS_SCALE && Math.abs(sy - 1) < FLIP_EPS_SCALE) {
            return;
        }
        el.style.transformOrigin = '0 0';
        el.style.transition = 'none';
        el.style.transform = `translate(${dx}px, ${dy}px) scale(${sx}, ${sy})`;
        targets.push(el);
    });
    targets.forEach(el => el.getBoundingClientRect());
    return targets;
}

export function playFlipForward(targets: HTMLElement[]): void {
    targets.forEach(el => {
        el.classList.add(FLIPPING_CLASS);
        el.style.transition = '';
        el.style.transform = '';
        const handler = (ev: TransitionEvent): void => {
            if (ev.propertyName !== 'transform') {
                return;
            }
            el.removeEventListener('transitionend', handler);
            el.removeEventListener('transitioncancel', handler);
            el.classList.remove(FLIPPING_CLASS);
            el.style.transformOrigin = '';
        };
        el.addEventListener('transitionend', handler);
        el.addEventListener('transitioncancel', handler);
    });
}
