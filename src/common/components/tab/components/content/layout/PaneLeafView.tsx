import {Box} from '@mui/material';
import {PaneBar} from '../PaneBar';
import {PaneSlot} from '../PaneSlot';
import {ReactNode, useCallback} from 'react';
import {PaneDropOverlay} from './PaneDropOverlay';
import {useEffect, useRef, useState} from 'react';
import {useTabContext} from '../../../hooks/useTabContext';
import {EVENT_TYPE, type PaneConfig, type PaneLeaf} from '../../../core/types';
import {PANE_DRAG_MODE, PHASE_TYPE, type PaneDragState} from '../../../model/PaneDrag';
import {computeSnapSide, computeDropRect, type DropRect} from '../../../util/splitDrag';

interface PaneLeafViewProps {
    leaf: PaneLeaf;
    panes: PaneConfig[];
    showTitle: boolean;
    focusedPaneId: string | undefined;
    paneHosts: Map<string, HTMLDivElement>;
}

export function PaneLeafView(props: PaneLeafViewProps): ReactNode {
    const {manager} = useTabContext();
    const activeSideRef = useRef<number | null>(null);
    const leafRef = useRef<HTMLDivElement | null>(null);
    const {leaf, panes, paneHosts, showTitle, focusedPaneId} = props;

    const host = paneHosts.get(leaf.paneId);
    const pane = panes.find(p => p.id === leaf.paneId);
    const isFocused = showTitle && leaf.paneId === focusedPaneId;
    const leafClassName = showTitle
        ? (isFocused ? 'tm-pane-leaf tm-pane-leaf--multi is-focused' : 'tm-pane-leaf tm-pane-leaf--multi')
        : undefined;
    const [dropRect, setDropRect] = useState<DropRect | null>(null);

    const clearOverlay = useCallback((): void => {
        if (activeSideRef.current === null) {
            return;
        }
        activeSideRef.current = null;
        setDropRect(null);
    }, []);

    const isDraggedOntoSelf = useCallback((next: PaneDragState): boolean => {
        if (next.dragMode === PANE_DRAG_MODE.PANE) {
            return next.tabId === leaf.paneId;
        }
        const draggedTab = manager.getTabs().find(t => t.id === next.tabId);
        return draggedTab ? draggedTab.panes.some(p => p.id === leaf.paneId) : false;
    }, [manager, leaf.paneId]);

    const canDropTabToCurrentPane = useCallback((next: PaneDragState): boolean => {
        if (next.dragMode !== PANE_DRAG_MODE.TAB) {
            return true;
        }
        if (!pane || !pane.id) {
            return false;
        }
        return manager.canDockTabToPane(next.tabId, pane.id);
    }, [manager, pane]);

    const commitDrop = useCallback((next: PaneDragState, side: number): void => {
        if (isDraggedOntoSelf(next)) {
            return;
        }
        if (!canDropTabToCurrentPane(next)) {
            return;
        }
        const el = leafRef.current;
        if (el) {
            manager.signalDropLanding(computeDropRect(el.getBoundingClientRect(), side));
        }
        if (next.dragMode === PANE_DRAG_MODE.PANE) {
            manager.dropPane(next.tabId, leaf.paneId, side);
            return;
        }
        manager.dockTabToPane(next.tabId, leaf.paneId, side);
    }, [canDropTabToCurrentPane, isDraggedOntoSelf, manager, leaf.paneId]);

    const handleDragEvent = useCallback((event: number, data: any): void => {
        if (EVENT_TYPE.DRAG != event) {
            return
        }
        const next = data as PaneDragState;
        if (next.phaseType === PHASE_TYPE.END) {
            const side = activeSideRef.current;
            if (side !== null) {
                commitDrop(next, side);
            }
            clearOverlay();
            return;
        }
        if (isDraggedOntoSelf(next)) {
            clearOverlay();
            return;
        }
        if (!canDropTabToCurrentPane(next)) {
            clearOverlay();
            return;
        }
        const el = leafRef.current;
        if (!el) {
            return;
        }
        const rect = el.getBoundingClientRect();
        const side = computeSnapSide(rect, next.clientX, next.clientY);
        if (side === activeSideRef.current) {
            return;
        }
        activeSideRef.current = side;
        setDropRect(side === null ? null : computeDropRect(rect, side));
    }, [canDropTabToCurrentPane, commitDrop, clearOverlay, isDraggedOntoSelf]);

    useEffect(() => {
        return manager.addEventListener(handleDragEvent);
    }, [manager, handleDragEvent]);

    if (!pane || !pane.id || !host) {
        return null;
    }
    return (
        <Box ref={leafRef} className={leafClassName} sx={{display: 'flex', flexDirection: 'column', width: '100%', height: '100%', position: 'relative'}}>
            {showTitle ? <PaneBar pane={pane}/> : null}
            <Box sx={{flex: 1, minHeight: 0}}>
                <PaneSlot paneId={pane.id} host={host}/>
            </Box>
            <PaneDropOverlay rect={dropRect}/>
        </Box>
    );
}
