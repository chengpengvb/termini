import {Box} from '@mui/material';
import {BarItem} from './BarItem.tsx';
import {BarTools} from './BarTools.tsx';
import type {MouseEvent as ReactMouseEvent} from 'react';
import {useTabContext} from '../../hooks/useTabContext.ts';
import {attachScrollInertia} from '../../util/scrollInertia.ts';
import type {TabManagerService} from '../../service/TabManagerService.ts';
import {BAR_TOOL_SLOT, EVENT_TYPE, type TabConfig} from '../../core/types.ts';
import {PANE_DRAG_MODE, PHASE_TYPE, type PaneDragState} from '../../model/PaneDrag.ts';
import {useCallback, useEffect, useRef, useState, type ReactNode, type RefObject} from 'react';
import {findNeighborTabId, isPointInStrip, orderedTabElements, resolveTabDropTarget} from '../../util/tabBarDrag.ts';

function countLeadingFixedTabs(tabs: TabConfig[], manager: TabManagerService): number {
    let count = 0;
    for (const tab of tabs) {
        const bar = tab.id ? manager.getTabBar(tab.id) : null;
        if (!bar || bar.canClose) {
            return count;
        }
        count += 1;
    }
    return count;
}

interface DragTracker {
    mutated: boolean;
    isDragging: boolean;
    originalActive: string | undefined;
}

interface DropInsert {
    index: number;
    width: number;
}

interface TabDragState {
    tabId: string;
    dropIndex: number;
}

interface BarItemsProps {
    tabs: TabConfig[];
    tabsRef: RefObject<TabConfig[]>;
    stripRef: RefObject<HTMLDivElement | null>;
    scrollerRef: RefObject<HTMLDivElement | null>;
    tabItemElsRef: RefObject<Map<string, HTMLElement>>;
    setItemEl: (tabId: string, el: HTMLElement | null) => void;
    onDragAreaMouseDown: (e: ReactMouseEvent<HTMLDivElement>) => void;
}

export function BarItems({tabs, stripRef, tabsRef, scrollerRef, onDragAreaMouseDown, tabItemElsRef, setItemEl}: BarItemsProps) {
    const {manager} = useTabContext();
    const tabDragRef = useRef<TabDragState | null>(null);
    const [dropInsert, setDropInsert] = useState<DropInsert | null>(null);
    const [activeTabId, setActiveTabId] = useState<string | undefined>(() => manager.getActiveTabId());
    const dragTrackerRef = useRef<DragTracker>({isDragging: false, mutated: false, originalActive: undefined});

    const resolveDrop = useCallback(
        (clientX: number) => {
            return resolveTabDropTarget(stripRef.current, orderedTabElements(tabsRef.current, tabItemElsRef.current), clientX);
        },
        [stripRef, tabsRef, tabItemElsRef],
    );

    const activateNeighborIfSourceActive = useCallback(
        (draggedTabId: string): void => {
            if (manager.getActiveTabId() !== draggedTabId) {
                return;
            }
            const neighborId = findNeighborTabId(manager.getTabs(), draggedTabId);
            if (!neighborId) {
                return;
            }
            manager.setActiveTabId(neighborId);
        },
        [manager],
    );

    const scheduleRestoreActive = useCallback((): void => {
        const tracker = dragTrackerRef.current;
        const original = tracker.originalActive;
        Promise.resolve().then(() => {
            tracker.isDragging = false;
            if (tracker.mutated || !original || manager.getActiveTabId() === original) {
                return;
            }
            manager.setActiveTabId(original);
        });
    }, [manager]);

    const handleDragStart = useCallback(
        (next: PaneDragState): void => {
            setDropInsert(null);
            dragTrackerRef.current = {isDragging: true, mutated: false, originalActive: manager.getActiveTabId()};
            if (next.dragMode != PANE_DRAG_MODE.TAB) {
                return;
            }
            activateNeighborIfSourceActive(next.tabId);
        },
        [manager, activateNeighborIfSourceActive],
    );

    const handleDragMove = useCallback(
        (next: PaneDragState): void => {
            const inStrip = isPointInStrip(stripRef.current, next.clientY);
            if (!inStrip) {
                setDropInsert(null);
                return;
            }
            const result = resolveDrop(next.clientX);
            if (!result || next.dragTarget.canMerge === false) {
                setDropInsert(null);
                return;
            }
            const dropIndex = manager.clampDropIndex(result.dropIndex);
            if (next.dragMode === PANE_DRAG_MODE.TAB) {
                tabDragRef.current = {tabId: next.tabId, dropIndex};
            }
            setDropInsert({index: dropIndex, width: Math.min(next.sourceWidth, 100)});
        },
        [manager, stripRef, resolveDrop],
    );

    const handleDragEnd = useCallback(
        (next: PaneDragState): void => {
            setDropInsert(null);
            const inStrip = isPointInStrip(stripRef.current, next.clientY);
            if (next.dragMode === PANE_DRAG_MODE.TAB && inStrip && tabDragRef.current) {
                manager.reorderTab(tabDragRef.current.tabId, tabDragRef.current.dropIndex);
                manager.setActiveTabId(next.tabId);
            } else if (next.dragMode === PANE_DRAG_MODE.PANE && inStrip) {
                const result = resolveDrop(next.clientX);
                if (result) {
                    manager.undockPaneToTab(next.tabId, result.dropIndex);
                }
            }
            scheduleRestoreActive();
            tabDragRef.current = null;
        },
        [manager, stripRef, resolveDrop, scheduleRestoreActive],
    );

    const handleManagerEvent = useCallback(
        (event: number, data: any): void => {
            if (EVENT_TYPE.ACTIVE === event) {
                setActiveTabId(data as string | undefined);
                return;
            }
            if (EVENT_TYPE.OPEN === event || EVENT_TYPE.CLOSE === event) {
                if (dragTrackerRef.current.isDragging) {
                    dragTrackerRef.current.mutated = true;
                }
                return;
            }
            if (EVENT_TYPE.DRAG !== event) {
                return;
            }
            const drag = data as PaneDragState;
            if (drag.phaseType === PHASE_TYPE.START) {
                handleDragStart(drag);
                return;
            }
            if (drag.phaseType === PHASE_TYPE.END) {
                handleDragEnd(drag);
                return;
            }
            handleDragMove(drag);
        },
        [handleDragStart, handleDragMove, handleDragEnd],
    );

    const fixedCount = countLeadingFixedTabs(tabs, manager);

    const renderSpacer = (index: number): ReactNode => {
        if (!dropInsert || dropInsert.index !== index) {
            return null;
        }
        return <span key={`spacer-${index}`} className="tm-tab-drop-spacer" style={{width: dropInsert.width}}/>;
    };

    const renderDivider = (index: number): ReactNode => {
        if (fixedCount === 0 || fixedCount >= tabs.length || index !== fixedCount - 1) {
            return null;
        }
        return <span key="fixed-divider" className="tm-tab-fixed-divider"/>;
    };

    const scrollActiveIntoView = useCallback((): void => {
        if (!activeTabId) {
            return;
        }
        if (!document.documentElement.classList.contains('tm-platform-mobile')) {
            return;
        }
        const el = tabItemElsRef.current.get(activeTabId);
        if (!el) {
            return;
        }
        el.scrollIntoView({inline: 'nearest', block: 'nearest', behavior: 'smooth'});
    }, [activeTabId, tabItemElsRef]);

    useEffect(() => {
        const el = scrollerRef.current;
        if (!el) {
            return;
        }
        return attachScrollInertia(el);
    }, [scrollerRef]);

    useEffect(() => {
        return manager.addEventListener(handleManagerEvent);
    }, [manager, handleManagerEvent]);

    useEffect(() => {
        scrollActiveIntoView();
        const frame = window.requestAnimationFrame(() => {
            scrollActiveIntoView();
        });
        return () => {
            window.cancelAnimationFrame(frame);
        };
    }, [scrollActiveIntoView, tabs]);

    return (
        <>
            <Box ref={scrollerRef} className="tm-tab-bar-items" onMouseDown={onDragAreaMouseDown}>
                {tabs.flatMap((tab, index) => [renderSpacer(index), <BarItem key={tab.id} tab={tab} isActive={tab.id === activeTabId} setItemEl={setItemEl}/>, renderDivider(index)])}
                {renderSpacer(tabs.length)}
            </Box>
            <BarTools slot={BAR_TOOL_SLOT.STRIP}/>
            <Box className="tm-tab-drag-tail" onMouseDown={onDragAreaMouseDown}/>
        </>
    );
}
