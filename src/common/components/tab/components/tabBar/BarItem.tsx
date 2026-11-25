import {Close} from '@mui/icons-material';
import {Box, ButtonBase} from '@mui/material';
import {EVENT_TYPE} from '../../core/types.ts';
import {useTabContext} from '../../hooks/useTabContext.ts';
import type {TabBar, TabConfig} from '../../core/types.ts';
import {ReactNode, useCallback, useEffect, useRef} from 'react';
import {useLongPressMenu} from '@/page/common/hooks/useLongPressMenu.ts';
import {PANE_DRAG_MODE, PHASE_TYPE, type PaneDragState} from '../../model/PaneDrag.ts';
import type {MouseEvent as ReactMouseEvent, PointerEvent as ReactPointerEvent} from 'react';

interface BarItemProps {
    tab: TabConfig;
    isActive: boolean;
    setItemEl: (tabId: string, el: HTMLElement | null) => void;
}

export function BarItem(props: BarItemProps): ReactNode {
    const {manager} = useTabContext();
    const isDropTargetRef = useRef(false);
    const {tab, isActive, setItemEl} = props;
    const buttonRef = useRef<HTMLButtonElement | null>(null);
    const tapRef = useRef<{x: number; y: number} | null>(null);

    const resolveTabId = (): string | null => {
        if (!tab.id) {
            return null;
        }
        return tab.id;
    };

    const resolveDisplay = (): TabBar | null => {
        const tabId = resolveTabId();
        if (!tabId) {
            return null;
        }
        return manager.getTabBar(tabId);
    };

    const resolveCanClose = (): boolean => {
        const display = resolveDisplay();
        return !!display && display.canClose && !!display.icon;
    };

    const longPress = useLongPressMenu(() => {
        const tabId = resolveTabId();
        if (!tabId || !resolveCanClose()) {
            return;
        }
        manager.closeTab(tabId);
    });

    const handleSelect = (): void => {
        if (longPress.consumeIfFired()) {
            return;
        }
        const tabId = resolveTabId();
        if (!tabId) {
            return;
        }
        manager.setActiveTabId(tabId);
        const el = buttonRef.current;
        if (el) {
            el.scrollIntoView({inline: 'nearest', block: 'nearest', behavior: 'smooth'});
        }
    };

    const handleClosePointerDown = (e: ReactPointerEvent<HTMLElement>): void => {
        e.stopPropagation();
    };

    const handleCloseClick = (e: ReactMouseEvent<HTMLElement>): void => {
        e.preventDefault();
        e.stopPropagation();
        const tabId = resolveTabId();
        if (!tabId) {
            return;
        }
        manager.closeTab(tabId);
    };

    const handleActionPointerDown = (e: ReactPointerEvent<HTMLElement>): void => {
        e.stopPropagation();
    };

    const handleActionClick = (e: ReactMouseEvent<HTMLElement>, onActionClick: () => void): void => {
        e.preventDefault();
        e.stopPropagation();
        onActionClick();
    };

    const handlePointerDown = (e: ReactPointerEvent<HTMLButtonElement>): void => {
        const tabId = resolveTabId();
        if ((e.pointerType === 'mouse' && e.button !== 0) || !tabId) {
            return;
        }
        if (e.pointerType !== 'mouse') {
            tapRef.current = {x: e.clientX, y: e.clientY};
            if (resolveCanClose()) {
                longPress.handlePointerDown(e);
            }
            return;
        }
        const rect = e.currentTarget.getBoundingClientRect();
        manager.armTabDrag(tab, rect, e.pointerId, e.clientX, e.clientY);
    };

    const handlePointerMove = (e: ReactPointerEvent<HTMLButtonElement>): void => {
        longPress.handlePointerMove(e);
        const start = tapRef.current;
        if (!start) {
            return;
        }
        const dx = e.clientX - start.x;
        const dy = e.clientY - start.y;
        if (dx * dx + dy * dy < 64) {
            return;
        }
        tapRef.current = null;
    };

    const handlePointerUp = (e: ReactPointerEvent<HTMLButtonElement>): void => {
        longPress.handlePointerEnd();
        if (e.pointerType === 'mouse') {
            return;
        }
        if (tapRef.current === null) {
            return;
        }
        tapRef.current = null;
        handleSelect();
    };

    const handlePointerCancel = (): void => {
        longPress.handlePointerEnd();
        tapRef.current = null;
    };

    const handleRef = (el: HTMLButtonElement | null): void => {
        buttonRef.current = el;
        const tabId = resolveTabId();
        if (!tabId) {
            return;
        }
        setItemEl(tabId, el);
    };

    const handleManagerEvent = useCallback((event: number, data: any): void => {
        if (EVENT_TYPE.DRAG !== event) {
            return;
        }
        const tabId = resolveTabId();
        if (!tabId) {
            return;
        }
        const drag = data as PaneDragState;
        const el = buttonRef.current;
        if (!el) {
            return;
        }
        if (drag.phaseType === PHASE_TYPE.START || drag.phaseType === PHASE_TYPE.END || drag.dragMode === PANE_DRAG_MODE.TAB) {
            isDropTargetRef.current = false;
            el.classList.remove('tm-is-drop-target');
            return;
        }
        const bar = manager.getTabBar(tabId);
        if (!bar || bar.canMerge === false || drag.dragTarget.canMerge === false) {
            return;
        }
        const rect = el.getBoundingClientRect();
        const isOver = drag.clientX >= rect.left && drag.clientX <= rect.right && drag.clientY >= rect.top && drag.clientY <= rect.bottom && drag.tabId !== tabId;
        if (isOver === isDropTargetRef.current) {
            return;
        }
        isDropTargetRef.current = isOver;
        el.classList.toggle('tm-is-drop-target', isOver);
        if (isOver) {
            manager.setActiveTabId(tabId);
        }
    }, [manager, tab.id]);

    useEffect(() => {
        const tabId = resolveTabId();
        if (!tabId) {
            return;
        }
        return manager.addEventListener(handleManagerEvent);
    }, [manager, handleManagerEvent, tab.id]);

    const tabId = resolveTabId();
    if (!tabId) {
        return null;
    }
    if (tab.panes.length === 0) {
        return null;
    }
    const display = resolveDisplay();
    if (!display) {
        return null;
    }
    const canShowClose = display.canClose && !!display.icon;
    const hasAction = !!display.actionIcon && !!display.onActionClick;
    const className = `tm-btn-highlight tm-tab-item${display.icon ? ' tm-has-icon' : ''}${canShowClose ? ' tm-can-close' : ''}${hasAction ? ' tm-has-action' : ''}${isActive ? ' tm-is-active' : ''}`;

    return (
        <ButtonBase
            ref={handleRef}
            data-tab-id={tabId}
            className={className}
            onClick={handleSelect}
            onPointerDown={handlePointerDown}
            onPointerMove={handlePointerMove}
            onPointerUp={handlePointerUp}
            onPointerCancel={handlePointerCancel}
            onPointerLeave={longPress.handlePointerEnd}
        >
            {display.icon ? (
                <Box className="tm-tab-icon-slot">
                    <Box className="tm-tab-icon" sx={{display: 'inline-flex', alignItems: 'center'}}>{display.icon}</Box>
                    {canShowClose ? (
                        <Box component="span" className="tm-tab-close tm-btn-tinted is-warning" tabIndex={-1} onPointerDown={handleClosePointerDown} onClick={handleCloseClick}>
                            <Close fontSize="inherit"/>
                        </Box>
                    ) : null}
                </Box>
            ) : null}
            <Box className={display.titleClassName ? `tm-tab-title ${display.titleClassName}` : 'tm-tab-title'} title={display.title}>{display.title}</Box>
            {hasAction ? (
                <Box component="span" className="tm-tab-action" tabIndex={-1} aria-label={display.actionTitle} onPointerDown={handleActionPointerDown} onClick={e => handleActionClick(e, display.onActionClick!)}>
                    {display.actionIcon}
                </Box>
            ) : null}
        </ButtonBase>
    );
}
