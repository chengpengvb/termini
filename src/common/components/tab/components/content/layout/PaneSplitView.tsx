import { useRef } from 'react';
import { Box } from '@mui/material';
import type { ReactNode } from 'react';
import { PaneTreeNode } from './PaneTreeNode';
import { PaneSplitDivider } from './PaneSplitDivider';
import { SPLIT_DIRECTION, type PaneConfig, type PaneSplit } from '../../../core/types';

interface PaneSplitViewProps {
    split: PaneSplit;
    showTitle: boolean;
    panes: PaneConfig[];
    focusedPaneId: string | undefined;
    paneHosts: Map<string, HTMLDivElement>;
}

export function PaneSplitView(props: PaneSplitViewProps): ReactNode {
    const { split, panes, paneHosts, showTitle, focusedPaneId } = props;
    const containerRef = useRef<HTMLDivElement | null>(null);
    const isHorizontal = split.direction === SPLIT_DIRECTION.HORIZONTAL;
    const cellClassName = showTitle ? 'tm-pane-cell tm-pane-cell--multi' : undefined;
    return (
        <Box ref={containerRef} className={showTitle ? 'tm-pane-split tm-pane-split--multi' : undefined} sx={{ display: 'flex', flexDirection: isHorizontal ? 'row' : 'column', width: '100%', height: '100%' }}>
            <Box className={cellClassName} sx={{ flex: split.ratio, minWidth: 0, minHeight: 0 }}>
                <PaneTreeNode node={split.first} panes={panes} paneHosts={paneHosts} showTitle={showTitle} focusedPaneId={focusedPaneId} />
            </Box>
            <PaneSplitDivider isHorizontal={isHorizontal} splitId={split.id} containerRef={containerRef} />
            <Box className={cellClassName} sx={{ flex: 1 - split.ratio, minWidth: 0, minHeight: 0 }}>
                <PaneTreeNode node={split.second} panes={panes} paneHosts={paneHosts} showTitle={showTitle} focusedPaneId={focusedPaneId} />
            </Box>
        </Box>
    );
}
