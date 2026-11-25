import type { TabBar } from "../core/types.ts";

export enum PANE_DRAG_MODE {
    PANE = 0,
    TAB = 1,
}

export const PHASE_TYPE = {
    START: 0,
    MOVE: 1,
    END: 2,
} as const;


export interface PaneDragState {
    tabId: string;
    dragTarget: TabBar;
    dragMode: number;
    phaseType: number;
    clientX: number;
    clientY: number;
    grabOffsetX: number;
    grabOffsetY: number;
    sourceWidth: number;
    sourceHeight: number;
}

export function calculateDragGhostSize(drag: PaneDragState): { width: number; height: number } {
    const width = Math.min(drag.sourceWidth, 200);
    const height = Math.min(drag.sourceHeight, 40);
    return {width, height};
}