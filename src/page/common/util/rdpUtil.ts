import {RDP_INPUT_KIND, type RdpInput} from "@/common/model/req/event/WsRdp.ts";
import {RDP_FRAME_KIND_JPEG, RDP_FRAME_KIND_PNG, RDP_FRAME_KIND_WEBP} from "@/common/model/resp/event/RespEventRdp.ts";

export const RDP_MOUSE_BUTTON = {
    LEFT: 0,
    RIGHT: 1,
    MIDDLE: 2,
    X1: 3,
    X2: 4,
} as const;

const WHEEL_LINE_HEIGHT = 40;
const WHEEL_STEP = 120;

const SCANCODE_MAP: Record<string, number> = {
    Escape: 0x01,
    Digit1: 0x02,
    Digit2: 0x03,
    Digit3: 0x04,
    Digit4: 0x05,
    Digit5: 0x06,
    Digit6: 0x07,
    Digit7: 0x08,
    Digit8: 0x09,
    Digit9: 0x0a,
    Digit0: 0x0b,
    Minus: 0x0c,
    Equal: 0x0d,
    Backspace: 0x0e,
    Tab: 0x0f,
    KeyQ: 0x10,
    KeyW: 0x11,
    KeyE: 0x12,
    KeyR: 0x13,
    KeyT: 0x14,
    KeyY: 0x15,
    KeyU: 0x16,
    KeyI: 0x17,
    KeyO: 0x18,
    KeyP: 0x19,
    BracketLeft: 0x1a,
    BracketRight: 0x1b,
    Enter: 0x1c,
    ControlLeft: 0x1d,
    KeyA: 0x1e,
    KeyS: 0x1f,
    KeyD: 0x20,
    KeyF: 0x21,
    KeyG: 0x22,
    KeyH: 0x23,
    KeyJ: 0x24,
    KeyK: 0x25,
    KeyL: 0x26,
    Semicolon: 0x27,
    Quote: 0x28,
    Backquote: 0x29,
    ShiftLeft: 0x2a,
    Backslash: 0x2b,
    KeyZ: 0x2c,
    KeyX: 0x2d,
    KeyC: 0x2e,
    KeyV: 0x2f,
    KeyB: 0x30,
    KeyN: 0x31,
    KeyM: 0x32,
    Comma: 0x33,
    Period: 0x34,
    Slash: 0x35,
    ShiftRight: 0x36,
    NumpadMultiply: 0x37,
    AltLeft: 0x38,
    Space: 0x39,
    CapsLock: 0x3a,
    F1: 0x3b,
    F2: 0x3c,
    F3: 0x3d,
    F4: 0x3e,
    F5: 0x3f,
    F6: 0x40,
    F7: 0x41,
    F8: 0x42,
    F9: 0x43,
    F10: 0x44,
    NumLock: 0x45,
    ScrollLock: 0x46,
    Numpad7: 0x47,
    Numpad8: 0x48,
    Numpad9: 0x49,
    NumpadSubtract: 0x4a,
    Numpad4: 0x4b,
    Numpad5: 0x4c,
    Numpad6: 0x4d,
    NumpadAdd: 0x4e,
    Numpad1: 0x4f,
    Numpad2: 0x50,
    Numpad3: 0x51,
    Numpad0: 0x52,
    NumpadDecimal: 0x53,
    IntlBackslash: 0x56,
    F11: 0x57,
    F12: 0x58,
    Help: 0x63,
    F13: 0x64,
    F14: 0x65,
    F15: 0x66,
    F16: 0x67,
    F17: 0x68,
    F18: 0x69,
    F19: 0x6a,
    F20: 0x6b,
    F21: 0x6c,
    F22: 0x6d,
    F23: 0x6e,
    F24: 0x6f,
    KanaMode: 0x70,
    Lang2: 0x71,
    Lang1: 0x72,
    IntlRo: 0x73,
    Convert: 0x79,
    NonConvert: 0x7b,
    IntlYen: 0x7d,
    NumpadComma: 0x7e,
};

const SCANCODE_EXTENDED_MAP: Record<string, number> = {
    MediaTrackPrevious: 0x10,
    MediaTrackNext: 0x19,
    NumpadEnter: 0x1c,
    ControlRight: 0x1d,
    AudioVolumeMute: 0x20,
    MediaPlayPause: 0x22,
    MediaStop: 0x24,
    AudioVolumeDown: 0x2e,
    AudioVolumeUp: 0x30,
    BrowserHome: 0x32,
    NumpadDivide: 0x35,
    PrintScreen: 0x37,
    AltRight: 0x38,
    Pause: 0x46,
    Home: 0x47,
    ArrowUp: 0x48,
    PageUp: 0x49,
    ArrowLeft: 0x4b,
    ArrowRight: 0x4d,
    End: 0x4f,
    ArrowDown: 0x50,
    PageDown: 0x51,
    Insert: 0x52,
    Delete: 0x53,
    MetaLeft: 0x5b,
    MetaRight: 0x5c,
    ContextMenu: 0x5d,
    Power: 0x5e,
    Sleep: 0x5f,
    BrowserSearch: 0x65,
    BrowserFavorites: 0x66,
    BrowserRefresh: 0x67,
    BrowserStop: 0x68,
    BrowserForward: 0x69,
    BrowserBack: 0x6a,
    LaunchMail: 0x6c,
    MediaSelect: 0x6d,
    LaunchApp1: 0x6e,
    LaunchApp2: 0x6f,
};

const MOUSE_BUTTON_MAP: Record<number, number> = {
    0: RDP_MOUSE_BUTTON.LEFT,
    1: RDP_MOUSE_BUTTON.MIDDLE,
    2: RDP_MOUSE_BUTTON.RIGHT,
    3: RDP_MOUSE_BUTTON.X1,
    4: RDP_MOUSE_BUTTON.X2,
};

export function resolveScancodeInput(code: string, pressed: boolean): RdpInput | null {
    const plain = SCANCODE_MAP[code];
    if (plain !== undefined) {
        return {kind: RDP_INPUT_KIND.SCANCODE, code: plain, pressed, extended: false};
    }
    const extended = SCANCODE_EXTENDED_MAP[code];
    if (extended !== undefined) {
        return {kind: RDP_INPUT_KIND.SCANCODE, code: extended, pressed, extended: true};
    }
    return null;
}

export function resolveUnicodeInput(key: string, pressed: boolean): RdpInput | null {
    if (Array.from(key).length !== 1) {
        return null;
    }
    const code = key.codePointAt(0);
    if (code === undefined || code > 0xffff) {
        return null;
    }
    return {kind: RDP_INPUT_KIND.UNICODE, code, pressed};
}

export function resolveMouseButton(button: number): number | null {
    const mapped = MOUSE_BUTTON_MAP[button];
    if (mapped === undefined) {
        return null;
    }
    return mapped;
}

export function resolveWheelDelta(delta: number, deltaMode: number): number {
    const pixels = deltaMode === WheelEvent.DOM_DELTA_PIXEL ? delta : delta * WHEEL_LINE_HEIGHT;
    const steps = Math.trunc((-pixels / WHEEL_LINE_HEIGHT) * WHEEL_STEP);
    return Math.max(-WHEEL_STEP, Math.min(WHEEL_STEP, steps));
}

export function resolveLockInput(event: KeyboardEvent): RdpInput {
    return {
        kind: RDP_INPUT_KIND.SYNC,
        kanaLock: false,
        numLock: event.getModifierState('NumLock'),
        capsLock: event.getModifierState('CapsLock'),
        scrollLock: event.getModifierState('ScrollLock'),
    };
}

const h264HexByte = (value: number): string => {
    const text = value.toString(16).toUpperCase();
    if (text.length === 1) {
        return `0${text}`;
    }
    if (text.length !== 2) {
        throw new Error(`H.264 byte out of range: ${value}`);
    }
    return text;
};

const visitAnnexBNals = (data: Uint8Array, visit: (offset: number) => boolean): void => {
    let index = 0;
    while (index + 4 < data.length) {
        let nal = -1;
        if (data[index] === 0 && data[index + 1] === 0 && data[index + 2] === 0 && data[index + 3] === 1) {
            nal = index + 4;
        } else if (data[index] === 0 && data[index + 1] === 0 && data[index + 2] === 1) {
            nal = index + 3;
        }
        if (nal >= 0 && nal < data.length) {
            if (!visit(nal)) {
                return;
            }
            index = nal + 1;
            continue;
        }
        index += 1;
    }
};

export function h264ChunkType(data: Uint8Array): EncodedVideoChunkType {
    let type: EncodedVideoChunkType = 'delta';
    visitAnnexBNals(data, (offset) => {
        if ((data[offset] & 0x1f) === 5) {
            type = 'key';
            return false;
        }
        return true;
    });
    return type;
}

export function h264CodecFromAnnexB(data: Uint8Array): string | null {
    let codec: string | null = null;
    visitAnnexBNals(data, (offset) => {
        if ((data[offset] & 0x1f) !== 7) {
            return true;
        }
        if (offset + 3 >= data.length) {
            throw new Error(`H.264 SPS too short: ${data.length - offset}`);
        }
        codec = `avc1.${h264HexByte(data[offset + 1])}${h264HexByte(data[offset + 2])}${h264HexByte(data[offset + 3])}`;
        return false;
    });
    return codec;
}

export function paintH264Frame(context: CanvasRenderingContext2D, frame: VideoFrame, x: number, y: number, width: number, height: number): void {
    const frameWidth = frame.displayWidth;
    const frameHeight = frame.displayHeight;
    if (frameWidth === width && frameHeight === height) {
        context.drawImage(frame, x, y);
        return;
    }
    if (x + width <= frameWidth && y + height <= frameHeight) {
        context.drawImage(frame, x, y, width, height, x, y, width, height);
        return;
    }
    throw new Error(`H.264 frame does not match dest: frame=${frameWidth}x${frameHeight} dest=${x},${y} ${width}x${height}`);
}

export function frameMime(kind: number): string {
    if (kind === RDP_FRAME_KIND_JPEG) {
        return 'image/jpeg';
    }
    if (kind === RDP_FRAME_KIND_PNG) {
        return 'image/png';
    }
    if (kind === RDP_FRAME_KIND_WEBP) {
        return 'image/webp';
    }
    throw new Error(`Unknown RDP frame kind: ${kind}`);
}

export function rdpFrameTimestamp(payload: ArrayBuffer): number {
    if (payload.byteLength < 8) {
        throw new Error(`RDP frame missing timestamp: ${payload.byteLength}`);
    }
    return Number(new DataView(payload).getBigUint64(0, true));
}

export function requireRdpCanvas(canvas: HTMLCanvasElement | null): HTMLCanvasElement {
    if (!canvas) {
        throw new Error('RdpView canvas is not mounted');
    }
    return canvas;
}

export function isRdpKeyboardTarget(target: EventTarget | null): boolean {
    const el = target instanceof Element ? target : document.activeElement;
    return el instanceof Element && el.closest('[data-tm-rdp="true"]') !== null;
}

// Pointer operations and text input belong to the same remote session. Moving
// focus to the canvas would dismiss the mobile keyboard (including composition).
export function focusRdpCanvas(canvas: HTMLCanvasElement): void {
    const active = document.activeElement;
    if (active instanceof HTMLTextAreaElement && !active.readOnly && active.inputMode !== 'none' && active.classList.contains('tm-rdp-ime') && active.parentElement === canvas.parentElement) {
        return;
    }
    canvas.focus({preventScroll: true});
}

export interface RdpContentBox {
    left: number;
    top: number;
    scale: number;
}

export function resolveRdpContentBox(canvas: HTMLCanvasElement): RdpContentBox | null {
    const rect = canvas.getBoundingClientRect();
    if (rect.width === 0 || rect.height === 0 || canvas.width === 0 || canvas.height === 0) {
        return null;
    }
    const scale = Math.min(rect.width / canvas.width, rect.height / canvas.height);
    return {
        left: rect.left + (rect.width - canvas.width * scale) / 2,
        top: rect.top + (rect.height - canvas.height * scale) / 2,
        scale,
    };
}

export function requireRdpContext(context: CanvasRenderingContext2D | null): CanvasRenderingContext2D {
    if (!context) {
        throw new Error('RdpView canvas context is not initialized');
    }
    return context;
}
