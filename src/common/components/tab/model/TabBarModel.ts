import type { TabConfig } from '../core/types';

export interface TabBarViewState {
    tabs: TabConfig[];
    activeTabId: string | undefined;
}
