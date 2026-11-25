import {Box} from '@mui/material';
import {createPortal} from 'react-dom';
import {useVirtualizer} from '@tanstack/react-virtual';
import {DragState} from "@/page/common/dto/sftp/SftpDrag.ts";
import type {FileEntry} from "@/common/model/dto/FileEntry.ts";
import {usePlatformMobile} from '@/common/context/usePlatformMobile';
import {useLongPressMenu} from '@/page/common/hooks/useLongPressMenu.ts';
import type {DragFromSelection} from "@/page/common/dto/sftp/SftpDrag.ts";
import {EntryRow} from '@/page/protocol/sftp/transfer/part/body/EntryRow.tsx';
import {BodyEntriesVirtualHandle} from '@/page/common/dto/sftp/BodyEntriesHandle.ts';
import {resolveDragSelectNames, resolveSftpEntryRowHeight} from '@/page/common/util/sftpUtil.ts';
import {forwardRef, MouseEvent, PointerEvent as ReactPointerEvent, useCallback, useEffect, useImperativeHandle, useLayoutEffect, useRef, useState} from 'react';

const SFTP_DRAG_EDGE_ZONE = 40;
const SFTP_AUTO_SCROLL_SPEED = 8;

export interface BodyEntriesVirtualProps {
    entries: FileEntry[];
    selectedNames: Set<string>;
    selectionMode: boolean;
    onClearSelection: () => void;
    onEntryDoubleClick: (entry: FileEntry) => void;
    onDragSelect: (names: string[], append: boolean) => void;
    onEntryClick: (name: string, multiSelect: boolean) => void;
    onBlankContextMenu: (event: MouseEvent<HTMLDivElement>) => void;
    onEntryContextMenu: (name: string, event: MouseEvent<HTMLDivElement>) => void;
}

export const BodyEntriesVirtual = forwardRef<BodyEntriesVirtualHandle, BodyEntriesVirtualProps>(function BodyEntriesVirtual(props, ref) {
    const mobileViewport = usePlatformMobile();
    const listRef = useRef<HTMLDivElement | null>(null);
    const dragStateRef = useRef<DragState | null>(null);
    const autoScrollRafRef = useRef<number | null>(null);
    const dragBoxRef = useRef<HTMLDivElement | null>(null);
    const itemRefs = useRef(new Map<string, HTMLElement>());
    const rowHeight = resolveSftpEntryRowHeight(mobileViewport);
    const [searchQuery, setSearchQuery] = useState<string | null>(null);
    const dragClientRef = useRef<{ clientX: number; clientY: number } | null>(null);
    const { entries, selectedNames, selectionMode, onEntryClick, onEntryDoubleClick, onEntryContextMenu, onDragSelect, onClearSelection, onBlankContextMenu } = props;
    const blankLongPress = useLongPressMenu((clientX, clientY) => {
        const event = {
            clientX,
            clientY,
            preventDefault: () => undefined,
            stopPropagation: () => undefined,
        } as MouseEvent<HTMLDivElement>;
        onBlankContextMenu(event);
    });

    const rowVirtualizer = useVirtualizer({
        overscan: 4,
        count: entries.length,
        useScrollendEvent: true,
        isScrollingResetDelay: 120,
        getScrollElement: () => listRef.current,
        estimateSize: () => rowHeight,
    });

    const registerItem = useCallback((name: string, element: HTMLElement | null): void => {
        if (!element) {
            itemRefs.current.delete(name);
            return;
        }
        itemRefs.current.set(name, element);
    }, []);

    function resolveViewCoords(clientX: number, clientY: number): { viewX: number; viewY: number; contentY: number } | null {
        const element = listRef.current;
        if (!element) {
            return null;
        }
        const rect = element.getBoundingClientRect();
        const viewX = clientX - rect.left;
        const viewY = clientY - rect.top;
        const contentY = viewY + element.scrollTop;
        return { viewX, viewY, contentY };
    }

    function setVisualDragSelected(name: string, selected: boolean): void {
        const element = itemRefs.current.get(name);
        if (!element) {
            return;
        }
        if (selected) {
            element.style.backgroundColor = 'var(--tm-bg-accent)';
            return;
        }
        element.style.backgroundColor = '';
    }

    function clearVisualDragSelection(state: DragState): void {
        state.visualNames.forEach(name => setVisualDragSelected(name, false));
        state.visualNames = new Set();
    }

    function syncVisualDragSelection(state: DragState): void {
        const names = resolveDragSelectNames(entries, state.startContentY, state.currentContentY, rowHeight);
        const nextNames = new Set(names);
        state.visualNames.forEach(name => {
            if (!nextNames.has(name)) {
                setVisualDragSelected(name, false);
            }
        });
        nextNames.forEach(name => {
            if (!state.visualNames.has(name)) {
                setVisualDragSelected(name, true);
            }
        });
        state.visualNames = nextNames;
    }

    function updateDragBox(state: DragState): void {
        const element = dragBoxRef.current;
        const list = listRef.current;
        if (!element || !list) {
            return;
        }
        const rect = list.getBoundingClientRect();
        const boxLeft = rect.left + Math.min(state.startViewX, state.currentViewX);
        const boxTop = rect.top + Math.min(state.startViewY, state.currentViewY);
        const boxRight = rect.left + Math.max(state.startViewX, state.currentViewX);
        const boxBottom = rect.top + Math.max(state.startViewY, state.currentViewY);
        const clampedLeft = Math.max(rect.left, boxLeft);
        const clampedTop = Math.max(rect.top, boxTop);
        const clampedRight = Math.min(rect.right, boxRight);
        const clampedBottom = Math.min(rect.bottom, boxBottom);
        element.style.display = 'block';
        element.style.left = `${clampedLeft}px`;
        element.style.top = `${clampedTop}px`;
        element.style.width = `${Math.max(0, clampedRight - clampedLeft)}px`;
        element.style.height = `${Math.max(0, clampedBottom - clampedTop)}px`;
    }

    function hideDragBox(): void {
        dragBoxRef.current?.style.setProperty('display', 'none');
    }

    function cancelAutoScroll(): void {
        if (!autoScrollRafRef.current) {
            return;
        }
        cancelAnimationFrame(autoScrollRafRef.current);
        autoScrollRafRef.current = null;
    }

    function tickAutoScroll(): void {
        const list = listRef.current;
        const state = dragStateRef.current;
        const dragClient = dragClientRef.current;
        if (!list || !state || !dragClient) {
            autoScrollRafRef.current = null;
            return;
        }
        const rect = list.getBoundingClientRect();
        const viewY = dragClient.clientY - rect.top;
        const maxScrollTop = list.scrollHeight - list.clientHeight;
        let shouldContinue = false;
        if (viewY < SFTP_DRAG_EDGE_ZONE && list.scrollTop > 0) {
            list.scrollTop = Math.max(0, list.scrollTop - SFTP_AUTO_SCROLL_SPEED);
            shouldContinue = true;
        } else if (viewY > rect.height - SFTP_DRAG_EDGE_ZONE && list.scrollTop < maxScrollTop) {
            list.scrollTop = Math.min(maxScrollTop, list.scrollTop + SFTP_AUTO_SCROLL_SPEED);
            shouldContinue = true;
        }
        if (!shouldContinue) {
            autoScrollRafRef.current = null;
            return;
        }
        state.currentContentY = viewY + list.scrollTop;
        updateDragBox(state);
        syncVisualDragSelection(state);
        autoScrollRafRef.current = requestAnimationFrame(tickAutoScroll);
    }

    function getScrollTop(): number {
        return listRef.current?.scrollTop ?? 0;
    }

    function resolveMaxScrollTop(): number {
        const element = listRef.current;
        if (!element) {
            return 0;
        }
        const totalHeight = entries.length * rowHeight;
        return Math.max(0, totalHeight - element.clientHeight);
    }

    function setScrollTop(scrollTop: number): void {
        const element = listRef.current;
        if (!element) {
            return;
        }
        element.scrollTop = Math.min(Math.max(0, scrollTop), resolveMaxScrollTop());
    }

    function scrollToIndex(index: number): void {
        const element = listRef.current;
        if (!element) {
            return;
        }
        const visibleItems = rowVirtualizer.getVirtualItems();
        const targetItem = visibleItems.find(item => item.index === index);
        if (targetItem) {
            const viewportTop = element.scrollTop;
            const viewportBottom = viewportTop + element.clientHeight;
            const isStrictlyVisible = targetItem.start >= viewportTop && targetItem.end <= viewportBottom;
            if (isStrictlyVisible) {
                return;
            }
        }
        rowVirtualizer.scrollToIndex(index, { align: 'center', behavior: 'smooth' });
    }

    function resolveHitName(contentY: number): string | undefined {
        return resolveDragSelectNames(entries, contentY, contentY, rowHeight)[0];
    }

    function beginDragSelect(clientX: number, clientY: number, append: boolean, onFromSelection?: (ctx: DragFromSelection) => void): void {
        cancelAutoScroll();
        const coords = resolveViewCoords(clientX, clientY);
        if (!coords) {
            return;
        }
        const existing = dragStateRef.current;
        if (existing) {
            clearVisualDragSelection(existing);
        }
        dragStateRef.current = null;
        const hitName = append ? undefined : resolveHitName(coords.contentY);
        if (hitName && selectedNames.has(hitName)) {
            hideDragBox();
            onFromSelection?.({clientX, clientY, count: selectedNames.size});
            return;
        }
        dragClientRef.current = { clientX, clientY };
        dragStateRef.current = {
            append,
            visualNames: new Set(),
            startViewX: coords.viewX,
            startViewY: coords.viewY,
            currentViewX: coords.viewX,
            currentViewY: coords.viewY,
            startContentY: coords.contentY,
            currentContentY: coords.contentY,
        };
    }

    function updateDragSelect(clientX: number, clientY: number): void {
        const state = dragStateRef.current;
        if (!state) {
            return;
        }
        const coords = resolveViewCoords(clientX, clientY);
        if (!coords) {
            return;
        }
        state.currentViewX = coords.viewX;
        state.currentViewY = coords.viewY;
        state.currentContentY = coords.contentY;
        dragClientRef.current = { clientX, clientY };
        updateDragBox(state);
        syncVisualDragSelection(state);
        const list = listRef.current;
        if (!list) {
            return;
        }
        const rect = list.getBoundingClientRect();
        const viewY = clientY - rect.top;
        const maxScrollTop = list.scrollHeight - list.clientHeight;
        const nearTop = viewY < SFTP_DRAG_EDGE_ZONE && list.scrollTop > 0;
        const nearBottom = viewY > rect.height - SFTP_DRAG_EDGE_ZONE && list.scrollTop < maxScrollTop;
        if ((nearTop || nearBottom) && !autoScrollRafRef.current) {
            autoScrollRafRef.current = requestAnimationFrame(tickAutoScroll);
        }
    }

    function endDragSelect(clientX: number, clientY: number): void {
        cancelAutoScroll();
        dragClientRef.current = null;
        const state = dragStateRef.current;
        if (!state) {
            return;
        }
        const coords = resolveViewCoords(clientX, clientY);
        if (coords) {
            state.currentViewX = coords.viewX;
            state.currentViewY = coords.viewY;
            state.currentContentY = coords.contentY;
        }
        const names = resolveDragSelectNames(entries, state.startContentY, state.currentContentY, rowHeight);
        clearVisualDragSelection(state);
        dragStateRef.current = null;
        hideDragBox();
        onDragSelect(names, state.append);
    }

    function cancelDragSelect(): void {
        cancelAutoScroll();
        dragClientRef.current = null;
        const state = dragStateRef.current;
        if (state) {
            clearVisualDragSelection(state);
        }
        dragStateRef.current = null;
        hideDragBox();
    }

    function getSelectedNames(): string[] {
        return Array.from(selectedNames);
    }

    function isPointerOnEntryRow(target: EventTarget | null): boolean {
        return target instanceof Element && Boolean(target.closest('[data-entry-row="1"]'));
    }

    function handleListClick(event: MouseEvent<HTMLDivElement>): void {
        if (event.ctrlKey || event.metaKey) {
            return;
        }
        if (!(event.target instanceof Node)) {
            return;
        }
        const target = event.target;
        for (const element of itemRefs.current.values()) {
            if (element.contains(target)) {
                return;
            }
        }
        if (selectionMode) {
            return;
        }
        onClearSelection();
    }

    function handleBlankPointerDown(event: ReactPointerEvent<HTMLDivElement>): void {
        if (isPointerOnEntryRow(event.target)) {
            return;
        }
        blankLongPress.handlePointerDown(event);
    }

    function handleBlankPointerMove(event: ReactPointerEvent<HTMLDivElement>): void {
        blankLongPress.handlePointerMove(event);
    }

    function handleBlankPointerEnd(): void {
        blankLongPress.handlePointerEnd();
    }

    function clearSelection(): void {
        cancelDragSelect();
        onClearSelection();
    }

    useImperativeHandle(ref, () => {
        const handle: BodyEntriesVirtualHandle = {
            endDragSelect,
            scrollToIndex,
            getScrollTop,
            setScrollTop,
            clearSelection,
            beginDragSelect,
            updateDragSelect,
            cancelDragSelect,
            getSelectedNames,
            setSearchQuery,
        };
        return handle;
    }, [rowVirtualizer, entries, selectedNames, onDragSelect]);

    useLayoutEffect(() => {
        rowVirtualizer.measure();
    }, [rowHeight, rowVirtualizer]);

    useLayoutEffect(() => {
        const element = listRef.current;
        if (!element) {
            return;
        }
        const nextScrollTop = Math.min(element.scrollTop, resolveMaxScrollTop());
        if (nextScrollTop !== element.scrollTop) {
            element.scrollTop = nextScrollTop;
        }
    }, [entries.length, rowHeight]);

    useEffect(() => {
        setSearchQuery(null);
    }, [entries]);

    useEffect(() => {
        return () => {
            cancelAutoScroll();
        };
    }, []);

    return (
        <>
            <Box
                ref={listRef}
                className="tm-sftp-entries"
                onClick={handleListClick}
                onPointerDown={handleBlankPointerDown}
                onPointerMove={handleBlankPointerMove}
                onPointerUp={handleBlankPointerEnd}
                onPointerCancel={handleBlankPointerEnd}
                sx={{ overflow: 'auto', flex: 1, minHeight: 0, position: 'relative', userSelect: 'none' }}
            >
                <Box sx={{ height: `${rowVirtualizer.getTotalSize()}px`, width: '100%', position: 'relative' }}>
                    {rowVirtualizer.getVirtualItems().map(virtualRow => {
                        const entry = entries[virtualRow.index];
                        return (
                            <Box key={entry.name} ref={(element: HTMLDivElement | null) => registerItem(entry.name, element)} data-index={virtualRow.index} sx={{ position: 'absolute', top: 0, left: 0, width: '100%', transform: `translateY(${virtualRow.start}px)` }}>
                                <EntryRow entry={entry} selected={selectedNames.has(entry.name)} selectionMode={selectionMode} searchQuery={searchQuery} onEntryClick={onEntryClick} onEntryDoubleClick={onEntryDoubleClick} onEntryContextMenu={onEntryContextMenu} />
                            </Box>
                        );
                    })}
                </Box>
            </Box>
            {createPortal(<Box ref={dragBoxRef} className="tm-sftp-drag-box" />, document.body)}
        </>
    );
});
