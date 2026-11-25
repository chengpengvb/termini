import type { WindowSize } from "../../dto/WindowSize.ts";

export interface SerialOpen {
  portName: string;
  size: WindowSize;
}

export interface SerialOpenEvent {
  data: SerialOpen;
}
