import {createContext} from 'react';
import type {TabManagerService} from "../service/TabManagerService.ts";

export interface TabContextValue {
    manager: TabManagerService;
}

export const TabContext = createContext<TabContextValue | null>(null);
