import {type RefObject, useEffect, useRef} from 'react';
import {useRdpZoom} from '@/page/common/hooks/useRdpZoom.ts';
import {focusRdpCanvas, RDP_MOUSE_BUTTON} from '@/page/common/util/rdpUtil.ts';
import {type RdpFocus, type RdpInput} from '@/common/model/req/event/WsRdp.ts';
import {type CursorPoint, useRdpPointer} from '@/page/common/hooks/useRdpPointer.ts';

const MOVE_SLOP = 8;
const LONG_PRESS_MS = 500;
const DOUBLE_TAP_MS = 300;
const DOUBLE_TAP_DIST = 24;
const WHEEL_DISTANCE = 24;

interface TouchState extends CursorPoint {
    startX: number;
    startY: number;
    moved: boolean;
    interrupted: boolean;
}
interface TapMark extends CursorPoint {time: number}
interface MultiState {
    x: number;
    y: number;
    distance: number;
    wheelY: number;
    wheel: number;
    kind: 'pending' | 'zoom' | 'wheel';
    moved: boolean;
    rightClick: boolean;
}
function pairState(points: Iterable<CursorPoint>) {
    const [a, b] = [...points];
    if (!a || !b) {
        return null;
    }
    return {x: (a.x + b.x) / 2, y: (a.y + b.y) / 2, distance: Math.hypot(a.x - b.x, a.y - b.y)};
}

export function useRdpTouch(canvasRef: RefObject<HTMLCanvasElement | null>, sendEvents: (events: RdpInput[]) => void, pointerRef: RefObject<HTMLElement | null>, enabled: boolean, touchMode: boolean, onFocusChange: (focus: RdpFocus | null) => void) {
    const touchesRef = useRef(new Map<number, TouchState>());
    const multiRef = useRef<MultiState | null>(null);
    const draggingRef = useRef(false);
    const secondTapRef = useRef(false);
    const downStampRef = useRef(0);
    const lastTapRef = useRef<TapMark | null>(null);
    const multiFrameRef = useRef(0);
    const modeRef = useRef(touchMode);
    const pointer = useRdpPointer(canvasRef, sendEvents, pointerRef);
    const zoom = useRdpZoom(canvasRef, () => {
        pointer.invalidateLayout();
        pointer.renderCursor();
    }, onFocusChange);
    modeRef.current = touchMode;

    const releaseDrag = (): void => {
        if (draggingRef.current) {
            draggingRef.current = false;
            pointer.pressButton(RDP_MOUSE_BUTTON.LEFT, false);
        }
    };
    const cancelGesture = (): void => {
        cancelAnimationFrame(multiFrameRef.current);
        multiFrameRef.current = 0;
        releaseDrag();
        touchesRef.current.clear();
        multiRef.current = null;
        lastTapRef.current = null;
        secondTapRef.current = false;
        zoom.endPinch();
        zoom.cancelMotion(false);
        zoom.relayout();
    };
    const desktopPoint = (x: number, y: number): CursorPoint | null => {
        const canvas = canvasRef.current;
        const point = zoom.canvasPoint(x, y);
        if (!canvas || !point || point.x < 0 || point.y < 0 || point.x >= canvas.width || point.y >= canvas.height) {
            return null;
        }
        return point;
    };
    const click = (x: number, y: number, button: number): void => {
        if (modeRef.current) {
            const point = desktopPoint(x, y);
            if (point) {
                pointer.clickAt(point, button);
            }
        } else {
            pointer.clickButton(button);
        }
    };
    const handleDown = (event: PointerEvent): void => {
        const canvas = canvasRef.current;
        const host = canvas?.parentElement;
        if (!canvas || !host || event.pointerType === 'mouse' || canvas.width === 0 || canvas.height === 0 || (event.target !== canvas && event.target !== host)) {
            return;
        }
        event.preventDefault();
        host.setPointerCapture(event.pointerId);
        focusRdpCanvas(canvas);
        const interrupted = touchesRef.current.size === 0 ? zoom.beginPan(event.timeStamp) : false;
        touchesRef.current.set(event.pointerId, {x: event.clientX, y: event.clientY, startX: event.clientX, startY: event.clientY, moved: false, interrupted});
        if (touchesRef.current.size === 1 && !multiRef.current) {
            downStampRef.current = event.timeStamp;
            const last = lastTapRef.current;
            secondTapRef.current = last !== null && event.timeStamp - last.time >= 40 && event.timeStamp - last.time <= DOUBLE_TAP_MS &&
                Math.hypot(event.clientX - last.x, event.clientY - last.y) <= DOUBLE_TAP_DIST;
            lastTapRef.current = null;
            return;
        }
        releaseDrag();
        secondTapRef.current = false;
        lastTapRef.current = null;
        const pair = pairState(touchesRef.current.values());
        if (touchesRef.current.size === 2 && pair && !multiRef.current) {
            multiRef.current = {...pair, wheelY: pair.y, wheel: 0, kind: 'pending', moved: false, rightClick: true};
            zoom.beginPinch(touchesRef.current.values());
        } else if (multiRef.current) {
            multiRef.current.moved = true;
            multiRef.current.rightClick = false;
            zoom.endPinch();
        }
    };
    const processMulti = (): void => {
        multiFrameRef.current = 0;
        const multi = multiRef.current;
        const pair = pairState(touchesRef.current.values());
        if (!multi || !pair || touchesRef.current.size !== 2 || !multi.rightClick) {
            return;
        }
        const spread = Math.abs(pair.distance - multi.distance);
        const moved = Math.hypot(pair.x - multi.x, pair.y - multi.y);
        if (multi.kind === 'pending') {
            if (spread < MOVE_SLOP && moved < MOVE_SLOP) {
                return;
            }
            multi.moved = true;
            multi.kind = modeRef.current || (spread >= 12 && spread > moved * 0.6) ? 'zoom' : 'wheel';
        }
        if (multi.kind === 'wheel' && spread >= Math.max(16, multi.distance * 0.12)) {
            multi.kind = 'zoom';
            zoom.beginPinch(touchesRef.current.values());
        }
        if (multi.kind === 'zoom') {
            zoom.pinchTo(touchesRef.current.values());
        } else {
            multi.wheel += pair.y - multi.wheelY;
            while (Math.abs(multi.wheel) >= WHEEL_DISTANCE) {
                pointer.scrollNotch(multi.wheel > 0 ? 1 : -1);
                multi.wheel -= Math.sign(multi.wheel) * WHEEL_DISTANCE;
            }
        }
        multi.wheelY = pair.y;
    };
    const handleMove = (event: PointerEvent): void => {
        const touch = touchesRef.current.get(event.pointerId);
        if (!touch) {
            return;
        }
        event.preventDefault();
        const dx = event.clientX - touch.x;
        const dy = event.clientY - touch.y;
        touch.x = event.clientX;
        touch.y = event.clientY;
        const multi = multiRef.current;
        if (multi) {
            if (!multiFrameRef.current) {
                multiFrameRef.current = requestAnimationFrame(processMulti);
            }
            return;
        }
        const fromX = touch.x - touch.startX;
        const fromY = touch.y - touch.startY;
        const began = !touch.moved && Math.hypot(fromX, fromY) >= MOVE_SLOP;
        if (began) {
            touch.moved = true;
            if (secondTapRef.current) {
                if (modeRef.current) {
                    const start = desktopPoint(touch.startX, touch.startY);
                    if (!start) {
                        return;
                    }
                    pointer.moveTo(start);
                }
                draggingRef.current = true;
                pointer.pressButton(RDP_MOUSE_BUTTON.LEFT, true);
            }
        }
        if (!touch.moved) {
            return;
        }
        if (modeRef.current && !draggingRef.current) {
            zoom.panBy(began ? fromX : dx, began ? fromY : dy, event.timeStamp);
        } else if (modeRef.current) {
            const point = zoom.canvasPoint(touch.x, touch.y);
            if (point) {
                pointer.moveTo(point);
            }
        } else {
            const before = pointer.getCursor();
            pointer.moveCursor(began ? fromX : dx, began ? fromY : dy);
            const after = pointer.getCursor();
            if (before && after && canvasRef.current) {
                zoom.followCursor(before, after, canvasRef.current);
            }
        }
    };
    const handleUp = (event: PointerEvent): void => {
        const touch = touchesRef.current.get(event.pointerId);
        if (!touch) {
            return;
        }
        event.preventDefault();
        if (multiFrameRef.current) {
            cancelAnimationFrame(multiFrameRef.current);
            processMulti();
        }
        touchesRef.current.delete(event.pointerId);
        const multi = multiRef.current;
        if (multi) {
            zoom.endPinch();
            if (touchesRef.current.size === 0) {
                if (multi.rightClick && !multi.moved) {
                    click(multi.x, multi.y, RDP_MOUSE_BUTTON.RIGHT);
                } else if (multi.kind === 'zoom') {
                    zoom.finishGesture();
                }
                multiRef.current = null;
            }
            return;
        }
        if (draggingRef.current) {
            releaseDrag();
            zoom.cancelMotion(false);
            return;
        }
        if (touch.moved) {
            if (modeRef.current) {
                zoom.finishGesture(event.timeStamp);
            }
            return;
        }
        if (touch.interrupted) {
            zoom.finishGesture(event.timeStamp);
            return;
        }
        const held = !secondTapRef.current && event.timeStamp - downStampRef.current >= LONG_PRESS_MS;
        click(event.clientX, event.clientY, held ? RDP_MOUSE_BUTTON.RIGHT : RDP_MOUSE_BUTTON.LEFT);
        if (!held && !secondTapRef.current) {
            lastTapRef.current = {x: event.clientX, y: event.clientY, time: event.timeStamp};
        }
        secondTapRef.current = false;
    };

    useEffect(() => {
        cancelGesture();
        pointer.invalidateLayout();
    }, [touchMode]);

    useEffect(() => {
        const canvas = canvasRef.current;
        const host = canvas?.parentElement;
        if (!canvas || !host) {
            return;
        }
        const relayout = (): void => {
            cancelGesture();
            pointer.invalidateLayout();
        };
        const observer = new ResizeObserver(relayout);
        observer.observe(host);
        observer.observe(canvas);
        relayout();
        return () => observer.disconnect();
    }, [canvasRef]);

    useEffect(() => {
        const host = canvasRef.current?.parentElement;
        if (!enabled || !host) {
            return;
        }
        const cancel = (event: PointerEvent): void => {
            if (touchesRef.current.has(event.pointerId)) {
                cancelGesture();
            }
        };
        const hidden = (): void => {
            if (document.hidden) {
                cancelGesture();
            }
        };
        const block = (event: Event): void => event.preventDefault();
        const listeners: Array<[string, EventListener]> = [
            ['pointerdown', handleDown as EventListener], ['pointermove', handleMove as EventListener],
            ['pointerup', handleUp as EventListener], ['pointercancel', cancel as EventListener],
            ['lostpointercapture', cancel as EventListener], ['contextmenu', block], ['selectstart', block],
        ];
        for (const [name, listener] of listeners) {
            host.addEventListener(name, listener, {passive: false});
        }
        window.addEventListener('blur', cancelGesture);
        window.addEventListener('orientationchange', cancelGesture);
        document.addEventListener('visibilitychange', hidden);
        pointer.renderCursor();
        return () => {
            cancelGesture();
            for (const [name, listener] of listeners) {
                host.removeEventListener(name, listener);
            }
            window.removeEventListener('blur', cancelGesture);
            window.removeEventListener('orientationchange', cancelGesture);
            document.removeEventListener('visibilitychange', hidden);
        };
    }, [canvasRef, enabled]);

    return {cursorReady: pointer.cursorReady, moveCursor: pointer.moveCursor, scroll: pointer.scrollNotch,
        fit: zoom.fit, actualSize: zoom.actualSize};
}
