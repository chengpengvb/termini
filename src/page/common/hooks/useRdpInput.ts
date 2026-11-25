import {type RefObject, useEffect, useRef} from 'react';
import {RDP_INPUT_KIND, type RdpInput} from '@/common/model/req/event/WsRdp.ts';
import {focusRdpCanvas, requireRdpCanvas, resolveMouseButton, resolveRdpContentBox, resolveScancodeInput, resolveUnicodeInput, resolveWheelDelta} from '@/page/common/util/rdpUtil.ts';

export function useRdpInput(
    paneId: string,
    mobile: boolean,
    canvasRef: RefObject<HTMLCanvasElement | null>,
) {
    const moveRafRef = useRef<number | null>(null);
    const pressedCodesRef = useRef<Set<string>>(new Set());
    const pressedButtonsRef = useRef(new Map<number, {x: number; y: number}>());
    const pendingMoveRef = useRef<{x: number; y: number} | null>(null);
    const onInputCallbackRef = useRef<((events: RdpInput[]) => void) | null>(null);

    const emit = (events: RdpInput[]): void => {
        const callback = onInputCallbackRef.current;
        if (!callback || events.length === 0) {
            return;
        }
        callback(events);
    };

    const keyboardCode = (event: KeyboardEvent): string => {
        if (event.code && event.code !== 'Unidentified') {
            return event.code;
        }
        return event.key;
    };

    const flushPendingMove = (): void => {
        if (moveRafRef.current !== null) {
            cancelAnimationFrame(moveRafRef.current);
            moveRafRef.current = null;
        }
        const point = pendingMoveRef.current;
        if (point === null) {
            return;
        }
        pendingMoveRef.current = null;
        emit([{kind: RDP_INPUT_KIND.MOVE, x: point.x, y: point.y}]);
    };

    const toDesktopPoint = (event: MouseEvent, clamp = false): { x: number; y: number } | null => {
        const canvas = requireRdpCanvas(canvasRef.current);
        const box = resolveRdpContentBox(canvas);
        if (!box) {
            throw new Error(`RdpView canvas size is zero: bitmap=${canvas.width}x${canvas.height}`);
        }
        const x = (event.clientX - box.left) / box.scale;
        const y = (event.clientY - box.top) / box.scale;
        if (!clamp && (x < 0 || y < 0 || x >= canvas.width || y >= canvas.height)) {
            return null;
        }
        return {
            x: Math.max(0, Math.min(canvas.width - 1, Math.round(x))),
            y: Math.max(0, Math.min(canvas.height - 1, Math.round(y))),
        };
    };

    const handleKeyDown = (event: KeyboardEvent): void => {
        event.preventDefault();
        event.stopPropagation();
        const code = keyboardCode(event);
        const scancode = resolveScancodeInput(code, true);
        if (scancode) {
            pressedCodesRef.current.add(code);
            emit([scancode]);
            return;
        }
        const unicode = resolveUnicodeInput(event.key, true);
        if (!unicode) {
            return;
        }
        emit([unicode]);
    };

    const handleKeyUp = (event: KeyboardEvent): void => {
        event.preventDefault();
        event.stopPropagation();
        const code = keyboardCode(event);
        const scancode = resolveScancodeInput(code, false);
        if (scancode) {
            pressedCodesRef.current.delete(code);
            emit([scancode]);
            return;
        }
        const unicode = resolveUnicodeInput(event.key, false);
        if (!unicode) {
            return;
        }
        emit([unicode]);
    };

    const handleBlur = (): void => {
        const releases: RdpInput[] = [];
        for (const code of pressedCodesRef.current) {
            const scancode = resolveScancodeInput(code, false);
            if (scancode) {
                releases.push(scancode);
            }
        }
        pressedCodesRef.current.clear();
        for (const [button, point] of pressedButtonsRef.current) {
            releases.push({kind: RDP_INPUT_KIND.MOUSE, ...point, button, pressed: false});
        }
        pressedButtonsRef.current.clear();
        emit(releases);
    };

    const handleMouseMove = (event: MouseEvent): void => {
        const point = toDesktopPoint(event, pressedButtonsRef.current.size > 0);
        if (!point) {
            return;
        }
        for (const button of pressedButtonsRef.current.keys()) {
            pressedButtonsRef.current.set(button, point);
        }
        pendingMoveRef.current = point;
        if (moveRafRef.current !== null) {
            return;
        }
        moveRafRef.current = requestAnimationFrame(() => {
            moveRafRef.current = null;
            const point = pendingMoveRef.current;
            if (point === null) {
                return;
            }
            pendingMoveRef.current = null;
            emit([{kind: RDP_INPUT_KIND.MOVE, x: point.x, y: point.y}]);
        });
    };

    const handleMouseDown = (event: MouseEvent): void => {
        event.preventDefault();
        const canvas = requireRdpCanvas(canvasRef.current);
        focusRdpCanvas(canvas);
        const button = resolveMouseButton(event.button);
        const point = toDesktopPoint(event);
        if (button === null || !point) {
            return;
        }
        if (event instanceof PointerEvent) {
            canvas.setPointerCapture(event.pointerId);
        }
        flushPendingMove();
        pressedButtonsRef.current.set(button, point);
        emit([{kind: RDP_INPUT_KIND.MOUSE, x: point.x, y: point.y, button, pressed: true}]);
    };

    const ownsPointer = (event: MouseEvent, canvas: HTMLCanvasElement): boolean => {
        if (event.target === canvas) {
            return true;
        }
        return event instanceof PointerEvent && canvas.hasPointerCapture(event.pointerId);
    };

    const handleMouseUp = (event: MouseEvent): void => {
        const canvas = canvasRef.current;
        if (!canvas || !canvas.isConnected || !ownsPointer(event, canvas)) {
            return;
        }
        const rect = canvas.getBoundingClientRect();
        if (rect.width === 0 || rect.height === 0 || canvas.width === 0 || canvas.height === 0) {
            return;
        }
        event.preventDefault();
        const button = resolveMouseButton(event.button);
        if (button === null || !pressedButtonsRef.current.has(button)) {
            return;
        }
        flushPendingMove();
        const point = toDesktopPoint(event, true);
        if (!point) {
            return;
        }
        pressedButtonsRef.current.delete(button);
        emit([{kind: RDP_INPUT_KIND.MOUSE, x: point.x, y: point.y, button, pressed: false}]);
    };

    const handleContextMenu = (event: MouseEvent): void => {
        event.preventDefault();
    };

    const handleWheel = (event: WheelEvent): void => {
        event.preventDefault();
        flushPendingMove();
        const point = toDesktopPoint(event);
        if (!point) {
            return;
        }
        const vertical = resolveWheelDelta(event.deltaY, event.deltaMode);
        if (vertical !== 0) {
            emit([{kind: RDP_INPUT_KIND.WHEEL, x: point.x, y: point.y, delta: vertical, horizontal: false}]);
        }
        const horizontal = resolveWheelDelta(event.deltaX, event.deltaMode);
        if (horizontal !== 0) {
            emit([{kind: RDP_INPUT_KIND.WHEEL, x: point.x, y: point.y, delta: horizontal, horizontal: true}]);
        }
    };

    const bindCanvasEvents = (canvas: HTMLCanvasElement): (() => void) => {
        const onWindowKeyDown = (event: KeyboardEvent): void => {
            if (document.activeElement === canvas) {
                handleKeyDown(event);
            }
        };
        const onWindowKeyUp = (event: KeyboardEvent): void => {
            if (document.activeElement === canvas || pressedCodesRef.current.has(keyboardCode(event))) {
                handleKeyUp(event);
            }
        };
        canvas.addEventListener('blur', handleBlur);
        window.addEventListener('blur', handleBlur);
        window.addEventListener('keydown', onWindowKeyDown, true);
        window.addEventListener('keyup', onWindowKeyUp, true);
        if (!mobile) {
            canvas.addEventListener('wheel', handleWheel, {passive: false});
            window.addEventListener('pointerup', handleMouseUp);
            window.addEventListener('pointercancel', handleMouseUp);
            canvas.addEventListener('pointerdown', handleMouseDown);
            canvas.addEventListener('pointermove', handleMouseMove);
            canvas.addEventListener('contextmenu', handleContextMenu);
        }
        return () => {
            handleBlur();
            if (moveRafRef.current !== null) {
                cancelAnimationFrame(moveRafRef.current);
                moveRafRef.current = null;
            }
            pendingMoveRef.current = null;
            canvas.removeEventListener('blur', handleBlur);
            window.removeEventListener('blur', handleBlur);
            window.removeEventListener('keydown', onWindowKeyDown, true);
            window.removeEventListener('keyup', onWindowKeyUp, true);
            canvas.removeEventListener('wheel', handleWheel);
            window.removeEventListener('pointerup', handleMouseUp);
            window.removeEventListener('pointercancel', handleMouseUp);
            canvas.removeEventListener('pointerdown', handleMouseDown);
            canvas.removeEventListener('pointermove', handleMouseMove);
            canvas.removeEventListener('contextmenu', handleContextMenu);
        };
    };

    const sendText = (text: string): void => {
        const events: RdpInput[] = [];
        for (const ch of text) {
            const down = resolveUnicodeInput(ch, true);
            const up = resolveUnicodeInput(ch, false);
            if (down) {
                events.push(down);
            }
            if (up) {
                events.push(up);
            }
        }
        emit(events);
    };

    const sendKey = (event: KeyboardEvent): void => {
        handleKeyDown(event);
        window.setTimeout(() => {
            handleKeyUp(event);
        }, 0);
    };

    const sendCode = (code: string): void => {
        const down = resolveScancodeInput(code, true);
        const up = resolveScancodeInput(code, false);
        if (!down || !up) {
            throw new Error(`rdp keyboard unknown code: ${code}`);
        }
        emit([down, up]);
    };

    const onData = (cb: (events: RdpInput[]) => void): (() => void) => {
        onInputCallbackRef.current = cb;
        return () => {
            if (onInputCallbackRef.current === cb) {
                onInputCallbackRef.current = null;
            }
        };
    };

    const dispose = (): void => {
        if (moveRafRef.current !== null) {
            cancelAnimationFrame(moveRafRef.current);
            moveRafRef.current = null;
        }
        pendingMoveRef.current = null;
        pressedCodesRef.current.clear();
    };

    useEffect(() => {
        const canvas = canvasRef.current;
        if (!canvas) {
            return;
        }
        return bindCanvasEvents(canvas);
    }, [paneId, mobile]);

    return {onData, dispose, sendText, sendKey, sendCode, sendEvents: emit};
}
