export function wheelDeltaX(e: { deltaMode: number; deltaX: number; deltaY: number }, pageSizePx: number): number {
    const factor = e.deltaMode === 1 ? 16 : e.deltaMode === 2 ? pageSizePx : 1;
    const dx = e.deltaX * factor;
    const dy = e.deltaY * factor;
    return dx !== 0 ? dx : dy;
}

