import type { WindowSize } from "../../dto/WindowSize.ts";

export interface ShellOpen {
  cwd?: string;
  shellId: string;
  size: WindowSize;
  attachProcessId?: string;
}

export interface ShellOpenEvent {
  data: ShellOpen;
}