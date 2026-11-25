import {type RefObject, useRef, useState} from 'react';
import {RDP_INPUT_KIND, type RdpInput} from '@/common/model/req/event/WsRdp.ts';
import {type RdpContentBox, resolveRdpContentBox} from '@/page/common/util/rdpUtil.ts';

const WHEEL_NOTCH = 120;
const POINTER_SENSITIVITY = 1.3;

export interface CursorPoint {
    x: number;
    y: number;
}

interface CursorLayout {
    box: RdpContentBox;
    hostLeft: number;
    hostTop: number;
}

export function useRdpPointer(canvasRef: RefObject<HTMLCanvasElement | null>, sendEvents: (events: RdpInput[]) => void, pointerRef: RefObject<HTMLElement | null>) {
    const cursorReadyRef = useRef<boolean>(false);
    const cursorRef = useRef<CursorPoint | null>(null);
    const layoutRef = useRef<CursorLayout | null>(null);
    const [cursorReady, setCursorReady] = useState<boolean>(false);

    const resolveCanvas = (): HTMLCanvasElement | null => {
        const canvas = canvasRef.current;
        if (!canvas || canvas.width === 0 || canvas.height === 0) {
            return null;
        }
        return canvas;
    };

    const ensureCursor = (canvas: HTMLCanvasElement): CursorPoint => {
        const current = cursorRef.current;
        if (current) {
            return current;
        }
        const next = {x: Math.round(canvas.width / 2), y: Math.round(canvas.height / 2)};
        cursorRef.current = next;
        return next;
    };

    const clampCursor = (canvas: HTMLCanvasElement): CursorPoint => {
        const cursor = ensureCursor(canvas);
        cursor.x = Math.max(0, Math.min(canvas.width - 1, cursor.x));
        cursor.y = Math.max(0, Math.min(canvas.height - 1, cursor.y));
        return cursor;
    };

    const resolveLayout = (canvas: HTMLCanvasElement): CursorLayout | null => {
        const cached = layoutRef.current;
        if (cached) {
            return cached;
        }
        const host = canvas.parentElement;
        const box = resolveRdpContentBox(canvas);
        if (!host || !box) {
            return null;
        }
        const hostRect = host.getBoundingClientRect();
        const layout = {box, hostLeft: hostRect.left, hostTop: hostRect.top};
        layoutRef.current = layout;
        return layout;
    };

    const invalidateLayout = (): void => {
        layoutRef.current = null;
    };

    const renderCursor = (): void => {
        const canvas = resolveCanvas();
        const element = pointerRef.current;
        if (!canvas || !element) {
            return;
        }
        const layout = resolveLayout(canvas);
        if (!layout) {
            return;
        }
        const cursor = clampCursor(canvas);
        element.style.setProperty('--tm-rdp-cursor-x', `${layout.box.left - layout.hostLeft + cursor.x * layout.box.scale}px`);
        element.style.setProperty('--tm-rdp-cursor-y', `${layout.box.top - layout.hostTop + cursor.y * layout.box.scale}px`);
        if (cursorReadyRef.current) {
            return;
        }
        cursorReadyRef.current = true;
        setCursorReady(true);
    };

    const sendMove = (): void => {
        const cursor = cursorRef.current;
        if (!cursor) {
            return;
        }
        // 后端 Move 的 x/y 是 u16，带小数会整条输入反序列化失败被丢掉
        sendEvents([{kind: RDP_INPUT_KIND.MOVE, x: Math.round(cursor.x), y: Math.round(cursor.y)}]);
    };

    const moveCursor = (dxCss: number, dyCss: number): void => {
        const canvas = resolveCanvas();
        const layout = canvas ? resolveLayout(canvas) : null;
        if (!canvas || !layout) {
            return;
        }
        const cursor = ensureCursor(canvas);
        const nextX = cursor.x + (dxCss / layout.box.scale) * POINTER_SENSITIVITY;
        const nextY = cursor.y + (dyCss / layout.box.scale) * POINTER_SENSITIVITY;
        cursor.x = Math.max(0, Math.min(canvas.width - 1, nextX));
        cursor.y = Math.max(0, Math.min(canvas.height - 1, nextY));
        renderCursor();
        sendMove();
    };

    const pressButton = (button: number, pressed: boolean): void => {
        const canvas = resolveCanvas();
        const cursor = canvas ? clampCursor(canvas) : cursorRef.current;
        if (!cursor || (!canvas && pressed)) {
            return;
        }
        const x = Math.round(cursor.x);
        const y = Math.round(cursor.y);
        // 先补一个移动再按键，保证远端指针一定落在光标点上，不依赖上一次移动事件
        sendEvents(pressed ? [{kind: RDP_INPUT_KIND.MOVE, x, y}, {kind: RDP_INPUT_KIND.MOUSE, x, y, button, pressed}] : [{kind: RDP_INPUT_KIND.MOUSE, x, y, button, pressed}]);
        renderCursor();
    };

    // 按下和抬起必须放在同一条消息里, 分两条发会被后端并发处理成先抬后按, 远端按键就卡住了
    const clickButton = (button: number): void => {
        const canvas = resolveCanvas();
        if (!canvas) {
            return;
        }
        const cursor = clampCursor(canvas);
        const x = Math.round(cursor.x);
        const y = Math.round(cursor.y);
        sendEvents([
            {kind: RDP_INPUT_KIND.MOVE, x, y},
            {kind: RDP_INPUT_KIND.MOUSE, x, y, button, pressed: true},
            {kind: RDP_INPUT_KIND.MOUSE, x, y, button, pressed: false},
        ]);
        renderCursor();
    };

    const moveTo = (point: CursorPoint): void => {
        const canvas = resolveCanvas();
        if (!canvas) {
            return;
        }
        cursorRef.current = {
            x: Math.max(0, Math.min(canvas.width - 1, point.x)),
            y: Math.max(0, Math.min(canvas.height - 1, point.y)),
        };
        renderCursor();
        sendMove();
    };

    const clickAt = (point: CursorPoint, button: number): void => {
        const canvas = resolveCanvas();
        if (!canvas) {
            return;
        }
        cursorRef.current = {
            x: Math.max(0, Math.min(canvas.width - 1, point.x)),
            y: Math.max(0, Math.min(canvas.height - 1, point.y)),
        };
        clickButton(button);
    };

    const scrollNotch = (direction: number): void => {
        const canvas = resolveCanvas();
        if (!canvas) {
            return;
        }
        const cursor = ensureCursor(canvas);
        sendEvents([{kind: RDP_INPUT_KIND.WHEEL, x: Math.round(cursor.x), y: Math.round(cursor.y), delta: (direction > 0 ? 1 : -1) * WHEEL_NOTCH, horizontal: false}]);
    };

    const getCursor = (): CursorPoint | null => cursorRef.current;

    return {cursorReady, clickAt, moveTo, moveCursor, pressButton, clickButton, scrollNotch, renderCursor, invalidateLayout, getCursor};
}
