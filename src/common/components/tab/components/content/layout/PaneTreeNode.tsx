import type { ReactNode } from 'react';
import { PaneLeafView } from './PaneLeafView';
import { PaneSplitView } from './PaneSplitView';
import { PANE_NODE_KIND, type PaneConfig, type PaneLeaf, type PaneSplit } from '../../../core/types';

interface PaneTreeNodeProps {
    panes: PaneConfig[];
    showTitle: boolean;
    focusedPaneId: string | undefined;
    node: PaneLeaf | PaneSplit;
    paneHosts: Map<string, HTMLDivElement>;
}

export function PaneTreeNode(props: PaneTreeNodeProps): ReactNode {
    const { node, panes, paneHosts, showTitle, focusedPaneId } = props;
    if (node.kind === PANE_NODE_KIND.LEAF) {
        return (
            <PaneLeafView leaf={node as PaneLeaf} panes={panes} paneHosts={paneHosts} showTitle={showTitle} focusedPaneId={focusedPaneId} />
        );
    }
    return (
        <PaneSplitView split={node as PaneSplit} panes={panes} paneHosts={paneHosts} showTitle={showTitle} focusedPaneId={focusedPaneId} />
    );
}
