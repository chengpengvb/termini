import { SPLIT_SIDE } from '../core/types';

export const MIN_PANE_PX = 60;

export interface DropRect {
    left: number;
    top: number;
    width: number;
    height: number;
}

export function computeSnapSide(rect: DOMRect, clientX: number, clientY: number): number | null {
    const dx = (clientX - rect.left) / rect.width;
    const dy = (clientY - rect.top) / rect.height;
    if (dx < 0 || dx > 1 || dy < 0 || dy > 1) {
        return null;
    }
    const minDist = Math.min(dy, 1 - dy, dx, 1 - dx);
    if (minDist === dy) {
        return SPLIT_SIDE.TOP;
    }
    if (minDist === 1 - dy) {
        return SPLIT_SIDE.BOTTOM;
    }
    if (minDist === dx) {
        return SPLIT_SIDE.LEFT;
    }
    return SPLIT_SIDE.RIGHT;
}

export function computeDropRect(containerRect: DOMRect, side: number): DropRect {
    const { left, top, width, height } = containerRect;
    if (side === SPLIT_SIDE.TOP) {
        return { left, top, width, height: height / 2 };
    }
    if (side === SPLIT_SIDE.BOTTOM) {
        return { left, top: top + height / 2, width, height: height / 2 };
    }
    if (side === SPLIT_SIDE.LEFT) {
        return { left, top, width: width / 2, height };
    }
    return { left: left + width / 2, top, width: width / 2, height };
}

export function computeClampedRatio(rect: DOMRect, clientX: number, clientY: number, isHorizontal: boolean): number | null {
    const axisSize = isHorizontal ? rect.width : rect.height;
    const rawRatio = isHorizontal ? (clientX - rect.left) / axisSize : (clientY - rect.top) / axisSize;
    if (axisSize <= MIN_PANE_PX * 2) {
        return null;
    }
    const minRatio = MIN_PANE_PX / axisSize;
    const maxRatio = 1 - minRatio;
    if (rawRatio < minRatio) {
        return minRatio;
    }
    if (rawRatio > maxRatio) {
        return maxRatio;
    }
    return rawRatio;
}
