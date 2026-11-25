import type { TabBar } from "../core/types.ts";
import { PANE_DRAG_MODE, PHASE_TYPE, type PaneDragState } from "../model/PaneDrag.ts";

export function buildDragBase(dragMode: PANE_DRAG_MODE, id: string, dragTarget: TabBar, rect: DOMRect, startX: number, startY: number): PaneDragState {
    return {
        dragMode,
        tabId: id,
        dragTarget,
        clientX: startX,
        clientY: startY,
        sourceWidth: rect.width,
        sourceHeight: rect.height,
        phaseType: PHASE_TYPE.START,
        grabOffsetX: startX - rect.left,
        grabOffsetY: startY - rect.top,
    };
}