import type {Terminal} from '@xterm/xterm';

interface Point {x: number; y: number}

export function terminalTouchViewport(host: HTMLElement, getTerminal: () => Terminal | null,
    lineHeight: () => number, onFontChange?: (size: number, finished: boolean) => void) {
    const points = new Map<number, Point>();
    let pinch: {distance: number; font: number} | null = null;
    let blocked = false;
    let frame = 0;
    let velocity = 0;
    let lastTime = 0;
    let remainder = 0;
    let position = {x: 0, y: 0};
    const stop = (): void => {
        cancelAnimationFrame(frame);
        frame = 0;
        velocity = 0;
        lastTime = 0;
        remainder = 0;
    };
    const distance = (): number => {
        const [a, b] = [...points.values()];
        return a && b ? Math.hypot(a.x - b.x, a.y - b.y) : 0;
    };
    const scrollPixels = (pixels: number): void => {
        const terminal = getTerminal();
        if (!terminal) {
            return;
        }
        const height = lineHeight();
        if (terminal.buffer.active.type === 'alternate' || terminal.modes.mouseTrackingMode !== 'none') {
            const rect = host.getBoundingClientRect();
            terminal.element?.dispatchEvent(new WheelEvent('wheel', {
                bubbles: true, cancelable: true, deltaY: pixels, deltaMode: WheelEvent.DOM_DELTA_PIXEL,
                clientX: Math.max(rect.left + 1, Math.min(rect.right - 1, position.x)),
                clientY: Math.max(rect.top + 1, Math.min(rect.bottom - 1, position.y)),
            }));
            return;
        }
        remainder += pixels / height;
        const lines = Math.trunc(remainder);
        if (lines !== 0) {
            remainder -= lines;
            terminal.scrollLines(lines);
        }
    };
    const finishPinch = (): void => {
        if (pinch) {
            const terminal = getTerminal();
            if (terminal) {
                onFontChange?.(terminal.options.fontSize ?? pinch.font, true);
            }
        }
        pinch = null;
    };
    return {
        active: (): boolean => blocked,
        down: (event: PointerEvent): boolean => {
            if (event.pointerType === 'mouse') {
                return false;
            }
            stop();
            points.set(event.pointerId, {x: event.clientX, y: event.clientY});
            if (points.size < 2 && !blocked) {
                return false;
            }
            blocked = true;
            host.setPointerCapture(event.pointerId);
            if (points.size === 2 && !pinch) {
                pinch = {distance: distance(), font: getTerminal()?.options.fontSize ?? 15};
            }
            return true;
        },
        move: (event: PointerEvent): boolean => {
            if (points.has(event.pointerId)) {
                points.set(event.pointerId, {x: event.clientX, y: event.clientY});
            }
            if (!blocked) {
                return false;
            }
            const terminal = getTerminal();
            if (terminal && pinch && pinch.distance > 0 && points.size === 2) {
                const size = Math.max(8, Math.min(48, Math.round(pinch.font * distance() / pinch.distance)));
                if (size !== terminal.options.fontSize) {
                    terminal.options.fontSize = size;
                    onFontChange?.(size, false);
                }
            }
            return true;
        },
        end: (event: PointerEvent): boolean => {
            points.delete(event.pointerId);
            const handled = blocked;
            if (points.size === 0) {
                finishPinch();
                blocked = false;
            }
            return handled;
        },
        scroll: (pixels: number, x: number, y: number, time: number): void => {
            position = {x, y};
            const elapsed = lastTime ? Math.max(1, Math.min(50, time - lastTime)) : 16;
            velocity = velocity * 0.5 + pixels / elapsed * 0.5;
            lastTime = time;
            scrollPixels(pixels);
        },
        release: (time: number): void => {
            const terminal = getTerminal();
            if (!terminal || terminal.buffer.active.type === 'alternate' || terminal.modes.mouseTrackingMode !== 'none' ||
                !lastTime || time - lastTime > 80 || Math.abs(velocity) < 0.08) {
                stop();
                return;
            }
            let previous = performance.now();
            const started = previous;
            const tick = (now: number): void => {
                const elapsed = Math.min(32, now - previous);
                previous = now;
                scrollPixels(velocity * elapsed);
                velocity *= Math.exp(-elapsed / 140);
                if (now - started > 700 || Math.abs(velocity) < 0.03) {
                    stop();
                    return;
                }
                frame = requestAnimationFrame(tick);
            };
            frame = requestAnimationFrame(tick);
        },
        cancel: (): void => {
            stop();
            finishPinch();
            points.clear();
            blocked = false;
        },
    };
}
