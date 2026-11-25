import {WindowSize} from "@/common/model/dto/WindowSize.ts";

export interface TerminalScreen {
    text: string;
    cursorRow: number;
    cursorCol: number;
}

export interface TerminalHandle {
    dispose: () => void;
    input: (data: string) => void;
    getCwd: () => string | null;
    getScreen: () => TerminalScreen;
    getWindowSize: () => WindowSize;
    toggleSettingsPanel: () => void;
    init: (onLayoutLoad: () => void) => void;
    write: (data: string | Uint8Array) => void;
    onData: (cb: (data: string) => void) => () => void;
    onWindowResize: (cb: (size: WindowSize) => void) => () => void;
}