import {observeElementRect, type Rect, type Virtualizer} from '@tanstack/react-virtual';

type RowVirtualizer = Virtualizer<HTMLDivElement, Element>;

const reconciledRows = new WeakSet<Element>();

export function observeAttachedElementRect(instance: RowVirtualizer, cb: (rect: Rect) => void): (() => void) | undefined {
    let detached = false;
    return observeElementRect(instance, (rect: Rect) => {
        if (rect.width === 0 && rect.height === 0) {
            detached = true;
            return;
        }
        const reattached = detached;
        detached = false;
        cb(rect);
        if (reattached) {
            reconcileVirtualRows(instance);
        }
    });
}

export function measureVirtualRow(element: Element, entry: ResizeObserverEntry | undefined, instance: RowVirtualizer): number {
    const index = instance.indexFromElement(element);
    const cached = instance.itemSizeCache.get(instance.options.getItemKey(index));
    if (entry === undefined && cached !== undefined) {
        return cached;
    }
    const box = entry?.borderBoxSize?.[0];
    const size = box ? box.blockSize : element.getBoundingClientRect().height;
    if (size > 0) {
        return Math.ceil(size);
    }
    return cached ?? instance.options.estimateSize(index);
}

export function reconcileVirtualRows(instance: RowVirtualizer): void {
    const root = instance.scrollElement;
    if (!root) {
        return;
    }
    const nodes = root.querySelectorAll<HTMLElement>(`[${instance.options.indexAttribute}]`);
    for (let position = 0; position < nodes.length; position++) {
        const node = nodes[position];
        const index = instance.indexFromElement(node);
        if (index < 0 || index >= instance.options.count) {
            continue;
        }
        const key = instance.options.getItemKey(index);
        if (instance.elementsCache.get(key) !== node) {
            instance.measureElement(node);
        }
        if (!reconciledRows.has(node)) {
            reconciledRows.add(node);
            continue;
        }
        const size = Math.ceil(node.getBoundingClientRect().height);
        if (size > 0 && size !== instance.itemSizeCache.get(key)) {
            instance.resizeItem(index, size);
        }
    }
}
