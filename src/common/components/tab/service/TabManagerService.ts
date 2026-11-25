import type { ReactNode } from 'react';
import type { DropRect } from '../util/splitDrag';
import type { BaseService } from '@/service/BaseService.ts';
import type { BarToolConfig, PaneConfig, PaneToolConfig, TabBar, TabConfig } from '../core/types';

export interface TabManagerService extends BaseService {

    markReady(): void;

    getTabs(): TabConfig[];

    waitReady(): Promise<void>;

    isSidePanelVisible(): boolean;

    getBarTools(): BarToolConfig[];

    closeTab(tabId: string): void;

    closePane(paneId: string): void;

    openTab(config: TabConfig): void;

    getSidePanel(): ReactNode | null;

    setActiveTabId(tabId: string): void;

    removeBarTool(toolId: string): void;

    getTab(id: string): TabConfig | null;

    removePaneTool(toolId: string): void;

    getActiveTabId(): string | undefined;

    clampDropIndex(index: number): number;

    signalPaneOpen(paneId: string): void;

    getPane(id: string): PaneConfig | null;

    setSidePanel(content: ReactNode): void;

    setFocusedPaneId(paneId: string): void;

    signalDropLanding(rect: DropRect): void;

    getTabBar(tabId: string): TabBar | null;

    addBarTool(config: BarToolConfig): string;

    setSidePanelVisible(visible: boolean): void;

    addPaneTool(config: PaneToolConfig): string;

    getPaneTools(paneId: string): PaneToolConfig[];

    updatePaneBar(paneId: string, bar: TabBar): void;

    reorderTab(tabId: string, toIndex: number): void;

    setSplitRatio(splitId: string, ratio: number): void;

    undockPaneToTab(paneId: string, insertAt: number): void;

    canDockTabToPane(fromTabId: string, toPaneId: string): boolean;

    dropPane(fromPaneId: string, toPaneId: string, side: number): void;

    dockTabToPane(fromTabId: string, toPaneId: string, side: number): void;

    addPaneOpenListener(listener: (pane: PaneConfig) => void): () => void;

    setFocusedPaneListener(listener: ((tab: TabConfig) => void) | null): void;

    addEventListener(listener: (event: number, data: any) => void): () => void;

    addCloseGuardListener(listener: (dragMode: number, data: any) => boolean): () => void;

    armTabDrag(tab: TabConfig, rect: DOMRect, pointerId: number, startX: number, startY: number): void;

    armPaneDrag(pane: PaneConfig, rect: DOMRect, pointerId: number, startX: number, startY: number): void;
}
