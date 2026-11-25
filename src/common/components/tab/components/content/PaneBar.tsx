import type { ReactNode } from 'react';
import { PaneTools } from './PaneTools';
import { Close } from '@mui/icons-material';
import { Box, ButtonBase } from '@mui/material';
import type { PaneConfig } from '../../core/types';
import { useTabContext } from '../../hooks/useTabContext';
import type { MouseEvent as ReactMouseEvent, PointerEvent as ReactPointerEvent } from 'react';

interface PaneBarProps {
    pane: PaneConfig;
}

export function PaneBar(props: PaneBarProps): ReactNode {
    const { pane } = props;
    const { manager } = useTabContext();

    const handleClosePointerDown = (e: ReactPointerEvent<HTMLButtonElement>): void => {
        e.stopPropagation();
    };

    const handleCloseClick = (e: ReactMouseEvent<HTMLButtonElement>): void => {
        e.preventDefault();
        e.stopPropagation();
        if (!pane.id) {
            return;
        }
        manager.closePane(pane.id);
    };

    const handlePointerDown = (e: ReactPointerEvent<HTMLDivElement>): void => {
        if (e.button !== 0 || !pane.id) {
            return;
        }
        manager.setFocusedPaneId(pane.id);
        const rect = e.currentTarget.getBoundingClientRect();
        manager.armPaneDrag(pane, rect, e.pointerId, e.clientX, e.clientY);
    };

    return (
        <Box onPointerDown={handlePointerDown} className="tm-pane-bar">
            {pane.tabBar.icon ? (<Box className="tm-pane-bar-icon">{pane.tabBar.icon}</Box>) : null}
            <Box className={pane.tabBar.titleClassName ? `tm-pane-bar-title ${pane.tabBar.titleClassName}` : 'tm-pane-bar-title'}>{pane.tabBar.title}</Box>
            {pane.id ? <PaneTools paneId={pane.id}/> : null}
            {pane.tabBar.canClose ? (
                <ButtonBase className="tm-pane-close tm-btn-tinted is-warning" onPointerDown={handleClosePointerDown} onClick={handleCloseClick}>
                    <Close fontSize="inherit"/>
                </ButtonBase>
            ) : null}
        </Box>
    );
}
