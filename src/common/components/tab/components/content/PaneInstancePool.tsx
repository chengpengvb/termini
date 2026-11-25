import type { ReactNode } from 'react';
import { PaneInstance } from './PaneInstance';
import type { TabConfig } from '../../core/types';

interface PaneInstancePoolProps {
    tabs: TabConfig[];
    registerPaneHost: (paneId: string, el: HTMLDivElement) => (() => void);
}

export function PaneInstancePool(props: PaneInstancePoolProps): ReactNode {
    const { tabs, registerPaneHost } = props;
    return (
        <>
            {tabs.flatMap(t => t.panes).map(pane => {
                if (!pane.id) {
                    return null;
                }
                return (
                    <PaneInstance key={pane.id} pane={pane} registerPaneHost={registerPaneHost} />
                );
            })}
        </>
    );
}
