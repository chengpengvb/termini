import type { TabConfig } from '../core/types';

export interface TabDropResult {
    dropIndex: number;
    visualX: number;
}

export function isPointInStrip(strip: HTMLElement | null, clientY: number): boolean {
    if (!strip) {
        return false;
    }
    const rect = strip.getBoundingClientRect();
    return clientY >= rect.top && clientY <= rect.bottom;
}

export function orderedTabElements(tabs: TabConfig[], map: Map<string, HTMLElement>): HTMLElement[] {
    const list: HTMLElement[] = [];
    for (let i = 0; i < tabs.length; i++) {
        const id = tabs[i].id;
        if (!id) {
            continue;
        }
        const el = map.get(id);
        if (el) {
            list.push(el);
        }
    }
    return list;
}

export function resolveTabDropTarget(strip: HTMLElement | null, items: HTMLElement[], clientX: number): TabDropResult | null {
    if (!strip || items.length === 0) {
        return null;
    }
    const stripRect = strip.getBoundingClientRect();
    for (let i = 0; i < items.length; i++) {
        const rect = items[i].getBoundingClientRect();
        if (clientX < rect.left + rect.width / 2) {
            return { dropIndex: i, visualX: rect.left - stripRect.left };
        }
    }
    const lastRect = items[items.length - 1].getBoundingClientRect();
    return { dropIndex: items.length, visualX: lastRect.right - stripRect.left };
}

export function resolveHoveredTabId(map: Map<string, HTMLElement>, clientX: number): string | null {
    for (const [tabId, el] of map) {
        const rect = el.getBoundingClientRect();
        if (clientX >= rect.left && clientX <= rect.right) {
            return tabId;
        }
    }
    return null;
}

export function findNeighborTabId(tabs: TabConfig[], draggedTabId: string): string | undefined {
    const idx = tabs.findIndex(t => t.id === draggedTabId);
    if (idx < 0) {
        return undefined;
    }
    const right = tabs[idx + 1];
    if (right && right.id) {
        return right.id;
    }
    const left = tabs[idx - 1];
    if (left && left.id) {
        return left.id;
    }
    return undefined;
}
