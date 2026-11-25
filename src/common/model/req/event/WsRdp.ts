import type { WindowSize } from "../../dto/WindowSize.ts";

export const RDP_INPUT_KIND = {
  SYNC: "sync",
  MOVE: "move",
  MOUSE: "mouse",
  WHEEL: "wheel",
  UNICODE: "unicode",
  SCANCODE: "scancode",
} as const;

export interface RdpOpen {
  size: WindowSize;
  deviceId: string;
}

export interface RdpOpenEvent {
  data: RdpOpen;
}

export interface RdpInputScancode {
  kind: typeof RDP_INPUT_KIND.SCANCODE;
  code: number;
  pressed: boolean;
  extended: boolean;
}

export interface RdpInputUnicode {
  kind: typeof RDP_INPUT_KIND.UNICODE;
  code: number;
  pressed: boolean;
}

export interface RdpInputMouse {
  kind: typeof RDP_INPUT_KIND.MOUSE;
  x: number;
  y: number;
  button: number;
  pressed: boolean;
}

export interface RdpInputMove {
  kind: typeof RDP_INPUT_KIND.MOVE;
  x: number;
  y: number;
}

export interface RdpInputWheel {
  kind: typeof RDP_INPUT_KIND.WHEEL;
  x: number;
  y: number;
  delta: number;
  horizontal: boolean;
}

export interface RdpInputSync {
  kind: typeof RDP_INPUT_KIND.SYNC;
  scrollLock: boolean;
  numLock: boolean;
  capsLock: boolean;
  kanaLock: boolean;
}

export type RdpInput = RdpInputScancode | RdpInputUnicode | RdpInputMouse | RdpInputMove | RdpInputWheel | RdpInputSync;

export interface RdpInputEvent {
  data: RdpInput[];
}

export interface RdpVisibilityEvent {
  data: boolean;
}

export interface RdpFocus {
  left: number;
  top: number;
  right: number;
  bottom: number;
}

export interface RdpFocusEvent {
  data: RdpFocus | null;
}
