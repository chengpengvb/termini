import {useContext} from 'react';
import {PaneContext} from '../context/PaneContext';
import type {PaneContextValue} from '../context/PaneContext';

export function usePaneContext(): PaneContextValue {
    const ctx = useContext(PaneContext);
    if (!ctx) {
        throw new Error('usePaneContext must be used inside a pane content');
    }
    return ctx;
}

