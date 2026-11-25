import {WindowSize} from "@/common/model/dto/WindowSize.ts";
import {RdpFocus, RdpInput} from "@/common/model/req/event/WsRdp.ts";
import {RespRdpDesktopSize} from "@/common/model/resp/event/RespEventRdp.ts";

export interface RdpHandle {
    dispose: () => void;
    getWindowSize: () => WindowSize;
    setNetwork: (bars: number) => void;
    init: (onLayoutLoad: () => void) => void;
    setCursor: (payload: ArrayBuffer) => void;
    resizeSurface: (size: RespRdpDesktopSize) => void;
    drawFrame: (payload: ArrayBuffer) => Promise<number>;
    onData: (cb: (events: RdpInput[]) => void) => () => void;
    onFocusChange: (cb: (focus: RdpFocus | null) => void) => () => void;
    onWindowResize: (cb: (size: WindowSize) => void) => () => void;
    onVisibilityChange: (cb: (visible: boolean) => void) => () => void;
}
