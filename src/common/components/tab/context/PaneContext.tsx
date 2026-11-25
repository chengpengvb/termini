import {createContext} from 'react';
import type {PaneConfig} from '../core/types';

export interface PaneContextValue {
    paneId: string;
    pane: PaneConfig;
}

export const PaneContext = createContext<PaneContextValue | null>(null);

