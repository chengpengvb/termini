import type {ReactNode} from 'react';
import {createPortal} from 'react-dom';
import {memo, useEffect, useState} from 'react';
import type {PaneConfig} from '../../core/types';
import {PaneContext} from '../../context/PaneContext';
import {useTabContext} from '../../hooks/useTabContext';

interface PaneInstanceProps {
    pane: PaneConfig;
    registerPaneHost: (paneId: string, el: HTMLDivElement) => (() => void);
}

export const PaneInstance = memo(function PaneInstanceInner(props: PaneInstanceProps): ReactNode {
    const {pane, registerPaneHost} = props;
    const {manager} = useTabContext();

    const [host] = useState<HTMLDivElement>(() => {
        const el = document.createElement('div');
        el.style.width = '100%';
        el.style.height = '100%';
        return el;
    });

    useEffect(() => {
        if (!pane.id) {
            return;
        }
        const unregister = registerPaneHost(pane.id, host);
        manager.signalPaneOpen(pane.id);
        return unregister;
    }, [pane.id, host, manager, registerPaneHost]);

    useEffect(() => {
        if (!pane.id) {
            return;
        }
        const onPointerDownCapture = (): void => {
            manager.setFocusedPaneId(pane.id!);
        };
        host.addEventListener('pointerdown', onPointerDownCapture, true);
        return () => host.removeEventListener('pointerdown', onPointerDownCapture, true);
    }, [host, manager, pane.id]);

    if (!pane.id) {
        return null;
    }
    return createPortal(
        <PaneContext.Provider value={{paneId: pane.id, pane}}>
            {pane.content}
        </PaneContext.Provider>,
        host,
    );
});

