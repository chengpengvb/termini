import {Box} from '@mui/material';
import type {ReactNode} from 'react';
import {useTabContext} from '../hooks/useTabContext';
import {PaneActiveLayout} from './content/PaneActiveLayout';
import {PaneInstancePool} from './content/PaneInstancePool';
import {useCallback, useEffect, useLayoutEffect, useRef, useState} from 'react';
import {applyFlipInverse, playFlipForward, snapshotElementRects} from '../util/flipAnimation';
import {EVENT_TYPE, TAB_LAYOUT_REASON, type TabConfig, type TabLayoutEventData} from '../core/types';

export function TabContent(): ReactNode {
    const {manager} = useTabContext();
    const flipSnapshotRef = useRef<Map<string, DOMRect> | null>(null);
    const paneHostsRef = useRef<Map<string, HTMLDivElement>>(new Map());
    const [tabs, setTabs] = useState<TabConfig[]>(() => manager.getTabs());
    const [paneHosts, setPaneHosts] = useState<Map<string, HTMLDivElement>>(() => new Map());
    const [activeTabId, setActiveTabId] = useState<string | undefined>(() => manager.getActiveTabId());

    const registerPaneHost = useCallback((paneId: string, el: HTMLDivElement): (() => void) => {
        const addHost = (prev: Map<string, HTMLDivElement>): Map<string, HTMLDivElement> => {
            if (prev.get(paneId) === el) {
                return prev;
            }
            const next = new Map(prev).set(paneId, el);
            paneHostsRef.current = next;
            return next;
        };
        const removeHost = (prev: Map<string, HTMLDivElement>): Map<string, HTMLDivElement> => {
            if (prev.get(paneId) !== el) {
                return prev;
            }
            const next = new Map(prev);
            next.delete(paneId);
            paneHostsRef.current = next;
            return next;
        };
        setPaneHosts(addHost);
        return () => setPaneHosts(removeHost);
    }, []);

    const captureFlipSnapshot = (): void => {
        flipSnapshotRef.current = snapshotElementRects(paneHostsRef.current);
    };

    useEffect(() => {
        const unsubscribe = manager.addEventListener((event: number, data: any) => {
            if (EVENT_TYPE.OPEN === event || EVENT_TYPE.CLOSE === event) {
                captureFlipSnapshot();
                setTabs(data as TabConfig[]);
            } else if (EVENT_TYPE.LAYOUT === event) {
                const payload = data as TabLayoutEventData;
                if (payload.reason === TAB_LAYOUT_REASON.PANE_MUTATION) {
                    captureFlipSnapshot();
                }
                setTabs(payload.tabs);
            } else if (EVENT_TYPE.ACTIVE === event) {
                setActiveTabId(data as string | undefined);
            }
        });
        manager.markReady();
        return unsubscribe;
    }, [manager]);

    useLayoutEffect(() => {
        const snapshot = flipSnapshotRef.current;
        if (!snapshot) {
            return;
        }
        flipSnapshotRef.current = null;
        const targets = applyFlipInverse(paneHostsRef.current, snapshot);
        if (targets.length === 0) {
            return;
        }
        window.requestAnimationFrame(() => {
            playFlipForward(targets);
        });
    });

    return (
        <Box sx={{flex: 1, minHeight: 0, minWidth: 0, position: 'relative'}}>
            <PaneInstancePool tabs={tabs} registerPaneHost={registerPaneHost}/>
            <PaneActiveLayout tabs={tabs} activeTabId={activeTabId} paneHosts={paneHosts}/>
        </Box>
    );
}
