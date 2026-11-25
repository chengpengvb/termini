import { Box } from '@mui/material';
import type { ReactNode } from 'react';
import { PaneTreeNode } from './layout/PaneTreeNode';
import { PANE_NODE_KIND, type TabConfig } from '../../core/types';

interface PaneActiveLayoutProps {
    tabs: TabConfig[];
    activeTabId: string | undefined;
    paneHosts: Map<string, HTMLDivElement>;
}

export function PaneActiveLayout(props: PaneActiveLayoutProps): ReactNode {
    const { tabs, activeTabId, paneHosts } = props;
    const activeTab = tabs.find(t => t.id === activeTabId);
    if (!activeTab || !activeTab.paneTree) {
        return null;
    }
    const showTitle = activeTab.paneTree.kind === PANE_NODE_KIND.SPLIT;
    const className = showTitle ? 'tm-pane-active-layout tm-pane-root--multi' : 'tm-pane-active-layout';
    return (
        <Box key={activeTabId} className={className} sx={{ position: 'absolute', inset: 0 }}>
            <PaneTreeNode node={activeTab.paneTree} panes={activeTab.panes} paneHosts={paneHosts} showTitle={showTitle} focusedPaneId={activeTab.focusedPaneId} />
        </Box>
    );
}
