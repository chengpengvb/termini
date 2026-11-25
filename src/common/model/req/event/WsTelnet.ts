import type { WindowSize } from "../../dto/WindowSize.ts";

export interface TelnetOpen {
  size: WindowSize;
  deviceId: string;
}

export interface TelnetOpenEvent {
  data: TelnetOpen;
}
