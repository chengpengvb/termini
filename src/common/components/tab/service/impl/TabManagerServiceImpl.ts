import { injectable } from 'inversify';
import type { ReactNode } from 'react';
import { generateId } from "@/common/util/IdUtil.ts";
import { buildDragBase } from "../../util/tabDrag.ts";
import { isEmpty } from "@/common/util/StringUtil.ts";
import type { DropRect } from "../../util/splitDrag.ts";
import { armPointerDrag } from "../../util/armMouseDrag.ts";
import type {TabManagerService} from "../TabManagerService.ts";
import { BaseServiceImpl } from "@/service/impl/BaseServiceImpl.ts";
import { PANE_DRAG_MODE, PHASE_TYPE, type PaneDragState } from "../../model/PaneDrag.ts";
import { applyRatioToTree, buildPaneTree, insertNodeNextTo, removeLeafFromTree } from "../../util/paneTree.ts";
import { EVENT_TYPE, PANE_NODE_KIND, TAB_LAYOUT_REASON, type BarToolConfig, type PaneConfig, type PaneLeaf, type PaneToolConfig, type TabBar, type TabConfig, type TabLayoutEventData } from "../../core/types.ts";

@injectable()
export class TabManagerServiceImpl extends BaseServiceImpl implements TabManagerService {

    private dragFrame: number = 0;
    private tabs: TabConfig[] = [];
    private barTools: BarToolConfig[] = [];
    private paneTools: PaneToolConfig[] = [];
    private dragMovePending: boolean = false;
    private sidePanel: ReactNode | null = null;
    private sidePanelVisible: boolean = false;
    private readonly readyPromise: Promise<void>;
    private readonly activeHistory: string[] = [];
    private readonly TAB_DRAG_START_DISTANCE_PX = 5;
    private dragDispose: (() => void) | null = null;
    private readyResolve: (() => void) | null = null;
    private activeTabId: string | undefined = undefined;
    private readonly paneOpenPending: Map<string, PaneConfig> = new Map();
    private focusedPaneListener: ((tab: TabConfig) => void) | null = null;
    private readonly listeners: Array<(event: number, data: any) => void> = [];
    private readonly paneOpenListeners: Set<(pane: PaneConfig) => void> = new Set();
    private readonly closeGuards: Set<(dragMode: number, data: any) => boolean> = new Set();
    private readonly paneOpenEmitted: Map<(pane: PaneConfig) => void, Set<string>> = new Map();

    constructor() {
        super();
        this.readyPromise = new Promise(resolve => {
            this.readyResolve = resolve;
        });
    }

    getSidePanel(): ReactNode | null {
        return this.sidePanel;
    }

    setSidePanel(content: ReactNode): void {
        if (this.sidePanel === content) {
            return;
        }
        this.sidePanel = content;
        this.notify(EVENT_TYPE.SIDE_PANEL, this.sidePanel);
    }

    isSidePanelVisible(): boolean {
        return this.sidePanelVisible;
    }

    setSidePanelVisible(visible: boolean): void {
        if (this.sidePanelVisible === visible) {
            return;
        }
        this.sidePanelVisible = visible;
        this.notify(EVENT_TYPE.SIDE_PANEL_VISIBLE, this.sidePanelVisible);
    }

    getBarTools(): BarToolConfig[] {
        return this.barTools;
    }

    addBarTool(config: BarToolConfig): string {
        if (isEmpty(config.id)) {
            config.id = generateId();
        }
        this.barTools = [...this.barTools, config];
        this.notify(EVENT_TYPE.BAR_TOOL, this.barTools);
        return config.id!;
    }

    removeBarTool(toolId: string): void {
        const exists = this.barTools.some(t => t.id === toolId);
        if (!exists) {
            return;
        }
        this.barTools = this.barTools.filter(t => t.id !== toolId);
        this.notify(EVENT_TYPE.BAR_TOOL, this.barTools);
    }

    getPaneTools(paneId: string): PaneToolConfig[] {
        return this.paneTools.filter(t => t.paneId === paneId);
    }

    addPaneTool(config: PaneToolConfig): string {
        if (isEmpty(config.id)) {
            config.id = generateId();
        }
        this.paneTools = [...this.paneTools, config];
        this.notify(EVENT_TYPE.PANE_TOOL, this.paneTools);
        return config.id!;
    }

    removePaneTool(toolId: string): void {
        const exists = this.paneTools.some(t => t.id === toolId);
        if (!exists) {
            return;
        }
        this.paneTools = this.paneTools.filter(t => t.id !== toolId);
        this.notify(EVENT_TYPE.PANE_TOOL, this.paneTools);
    }

    markReady(): void {
        if (this.readyResolve) {
            this.readyResolve();
            this.readyResolve = null;
        }
    }

    waitReady(): Promise<void> {
        return this.readyPromise;
    }

    getTabs(): TabConfig[] {
        return this.tabs;
    }

    getTab(id: string): TabConfig | null {
        const tab = this.tabs.find(item => item.id === id);
        if (!tab) {
            return null;
        }
        return tab;
    }

    getPane(id: string): PaneConfig | null {
        for (let i = 0; i < this.tabs.length; i += 1) {
            const tab = this.tabs[i];
            for (let j = 0; j < tab.panes.length; j += 1) {
                const pane = tab.panes[j];
                if (pane.id === id) {
                    return pane;
                }
            }
        }
        return null;
    }

    getActiveTabId(): string | undefined {
        return this.activeTabId;
    }

    getTabBar(tabId: string): TabBar | null {
        const tab = this.getTab(tabId);
        if (!tab) {
            return null;
        }
        return this.resolveTabBarDisplay(tab);
    }

    updatePaneBar(paneId: string, bar: TabBar): void {
        const pane = this.getPane(paneId);
        if (!pane) {
            return;
        }
        let changedTabId: string | null = null;
        const nextTabs = this.tabs.map(tab => {
            let paneChanged = false;
            const nextPanes = tab.panes.map(item => {
                if (item.id !== paneId) {
                    return item;
                }
                paneChanged = true;
                return { ...item, tabBar: bar };
            });
            if (!paneChanged) {
                return tab;
            }
            changedTabId = tab.id ?? null;
            return { ...tab, panes: nextPanes };
        });
        if (!changedTabId) {
            return;
        }
        this.tabs = nextTabs;
        if (this.focusedPaneListener) {
            const nextTab = this.getTab(changedTabId);
            if (nextTab) {
                this.focusedPaneListener(nextTab);
            }
        }
        this.notifyLayout(TAB_LAYOUT_REASON.TAB_BAR);
    }

    setFocusedPaneListener(listener: ((tab: TabConfig) => void) | null): void {
        this.focusedPaneListener = listener;
    }

    addCloseGuardListener(listener: (dragMode: number, data: any) => boolean): () => void {
        this.closeGuards.add(listener);
        return () => {
            this.closeGuards.delete(listener);
        };
    }

    addPaneOpenListener(listener: (pane: PaneConfig) => void): () => void {
        this.paneOpenListeners.add(listener);
        const emitted = new Set<string>();
        this.paneOpenEmitted.set(listener, emitted);
        this.paneOpenPending.forEach((pane, paneId) => {
            emitted.add(paneId);
            listener(pane);
        });
        return () => {
            this.paneOpenListeners.delete(listener);
            this.paneOpenEmitted.delete(listener);
        };
    }

    signalPaneOpen(paneId: string): void {
        const tab = this.tabs.find(t => t.panes.some(p => p.id === paneId));
        if (!tab) {
            return;
        }
        const pane = tab.panes.find(p => p.id === paneId);
        if (!pane) {
            return;
        }
        if (!this.paneOpenPending.has(paneId)) {
            this.paneOpenPending.set(paneId, pane);
        }
        this.paneOpenListeners.forEach(listener => {
            const emitted = this.paneOpenEmitted.get(listener);
            if (!emitted || emitted.has(paneId)) {
                return;
            }
            emitted.add(paneId);
            listener(pane);
        });
    }

    private canClose(dragMode: number, data: any): boolean {
        for (const guard of this.closeGuards) {
            if (!guard(dragMode, data)) {
                return false;
            }
        }
        return true;
    }

    closePane(paneId: string): void {
        this.paneOpenPending.delete(paneId);
        this.paneOpenEmitted.forEach(emitted => emitted.delete(paneId));
        const tab = this.tabs.find(t => t.panes.some(p => p.id === paneId));
        if (!tab || !tab.id) {
            return;
        }
        const pane = tab.panes.find(p => p.id === paneId);
        if (!pane) {
            return;
        }
        if (!this.canClose(PANE_DRAG_MODE.PANE, pane)) {
            return;
        }
        if (tab.panes.length <= 1) {
            this.closeTabInternal(tab.id);
            return;
        }
        if (!tab.paneTree) {
            return;
        }
        const remainingTree = removeLeafFromTree(tab.paneTree, paneId);
        if (!remainingTree) {
            return;
        }
        const remainingPanes = tab.panes.filter(p => p.id !== paneId);
        let nextFocused = tab.focusedPaneId;
        if (nextFocused === paneId) {
            const first = remainingPanes[0];
            if (first && first.id) {
                nextFocused = first.id;
            } else {
                nextFocused = undefined;
            }
        }
        this.tabs = this.tabs.map(t => t.id === tab.id ? { ...t, panes: remainingPanes, paneTree: remainingTree, focusedPaneId: nextFocused } : t);
        const nextTab = this.tabs.find(t => t.id === tab.id);
        if (nextTab && this.focusedPaneListener) {
            this.focusedPaneListener(nextTab);
        }
        this.notifyLayout(TAB_LAYOUT_REASON.PANE_MUTATION);
    }

    setFocusedPaneId(paneId: string): void {
        const tab = this.tabs.find(t => t.panes.some(p => p.id === paneId));
        if (!tab || !tab.id) {
            return;
        }
        const shouldUpdateFocus = tab.focusedPaneId !== paneId;
        const shouldUpdateActive = this.activeTabId !== tab.id;
        if (shouldUpdateFocus) {
            this.tabs = this.tabs.map(t => t.id === tab.id ? { ...t, focusedPaneId: paneId } : t);
        }
        const nextTab = this.tabs.find(t => t.id === tab.id);
        if (nextTab && this.focusedPaneListener) {
            this.focusedPaneListener(nextTab);
        }
        if (shouldUpdateActive) {
            this.activateTabId(tab.id);
        }
        if (shouldUpdateFocus) {
            this.notifyLayout(TAB_LAYOUT_REASON.FOCUS);
        }
    }

    private resolveDisplayPane(tab: TabConfig): PaneConfig | null {
        const panes = tab.panes;
        if (panes.length === 0) {
            return null;
        }
        let displayPane = panes[0];
        const focusedId = tab.focusedPaneId;
        if (focusedId) {
            const found = panes.find(p => p.id === focusedId);
            if (found) {
                displayPane = found;
            }
        }
        return displayPane;
    }

    private resolveTabBarDisplay(tab: TabConfig): TabBar | null {
        const pane = this.resolveDisplayPane(tab);
        if (!pane) {
            return null;
        }
        return pane.tabBar;
    }

    private resolvePaneById(tab: TabConfig, paneId: string): PaneConfig | null {
        const pane = tab.panes.find(item => item.id === paneId);
        if (!pane) {
            return null;
        }
        return pane;
    }

    private canTabMergeToPane(fromTab: TabConfig, toTab: TabConfig, toPaneId: string): boolean {
        const sourceBar = this.resolveTabBarDisplay(fromTab);
        const targetPane = this.resolvePaneById(toTab, toPaneId);
        if (!sourceBar || !targetPane) {
            return false;
        }
        return sourceBar.canMerge !== false && targetPane.tabBar.canMerge !== false;
    }

    openTab(config: TabConfig): void {
        this.ensureTabAndPaneIds(config);
        this.tabs = [...this.tabs, config];
        this.activateTabId(config.id);
        if (this.focusedPaneListener) {
            this.focusedPaneListener(config);
        }
        this.notify(EVENT_TYPE.OPEN, this.tabs);
    }

    closeTab(tabId: string): void {
        const tab = this.tabs.find(t => t.id === tabId);
        if (!tab || !tab.id) {
            return;
        }
        if (!this.canClose(PANE_DRAG_MODE.TAB, tab)) {
            return;
        }
        this.closeTabInternal(tabId);
    }

    private closeTabInternal(tabId: string): void {
        const idx = this.tabs.findIndex(t => t.id === tabId);
        if (idx < 0) {
            return;
        }
        const removed = this.tabs[idx];
        if (removed) {
            for (let i = 0; i < removed.panes.length; i += 1) {
                const paneId = removed.panes[i].id;
                if (paneId) {
                    this.paneOpenPending.delete(paneId);
                    this.paneOpenEmitted.forEach(emitted => emitted.delete(paneId));
                }
            }
        }
        this.forgetActiveHistory(tabId);
        this.tabs = this.tabs.filter(t => t.id !== tabId);
        if (this.activeTabId === tabId) {
            this.activeTabId = this.resolveHistoryActiveId(idx);
        }
        this.notify(EVENT_TYPE.CLOSE, this.tabs);
        this.notify(EVENT_TYPE.ACTIVE, this.activeTabId);
    }

    setActiveTabId(tabId: string): void {
        const tab = this.tabs.find(t => t.id === tabId);
        if (!tab) {
            return
        }
        this.activateTabId(tabId);
        if (this.focusedPaneListener) {
            this.focusedPaneListener(tab);
        }
    }

    armTabDrag(tab: TabConfig, rect: DOMRect, pointerId: number, startX: number, startY: number): void {
        if (isEmpty(tab.id)) {
            return;
        }
        if (tab.panes.length === 0) {
            return;
        }
        this.clearDragArm();
        const display = this.resolveTabBarDisplay(tab);
        const dragTarget = display ? display : tab.panes[0].tabBar;
        const state = buildDragBase(PANE_DRAG_MODE.TAB, tab.id!, dragTarget, rect, startX, startY);
        this.dragDispose = this.armTabDragState(state, pointerId, startX, startY);
    }

    armPaneDrag(pane: PaneConfig, rect: DOMRect, pointerId: number, startX: number, startY: number): void {
        if (isEmpty(pane.id)) {
            return;
        }
        this.clearDragArm();
        const state = buildDragBase(PANE_DRAG_MODE.PANE, pane.id!, pane.tabBar, rect, startX, startY);
        this.dragDispose = this.armTabDragState(state, pointerId, startX, startY);
    }

    addEventListener(cb: (event: number, data: any) => void): () => void {
        const fn = cb as (event: number, data: any) => void;
        this.listeners.push(fn);
        return () => {
            const idx = this.listeners.indexOf(fn);
            if (idx < 0) {
                return;
            }
            this.listeners.splice(idx, 1);
        };
    }

    private lockedPrefixCount(): number {
        let count = 0;
        for (const tab of this.tabs) {
            const bar = this.resolveTabBarDisplay(tab);
            if (!bar || bar.canMerge !== false) {
                return count;
            }
            count += 1;
        }
        return count;
    }

    clampDropIndex(index: number): number {
        const min = this.lockedPrefixCount();
        if (index < min) {
            return min;
        }
        if (index > this.tabs.length) {
            return this.tabs.length;
        }
        return index;
    }

    reorderTab(tabId: string, toIndex: number): void {
        const fromIdx = this.tabs.findIndex(t => t.id === tabId);
        if (fromIdx < 0) {
            return;
        }
        const fromBar = this.resolveTabBarDisplay(this.tabs[fromIdx]);
        if (!fromBar || fromBar.canMerge === false) {
            return;
        }
        const dropIndex = this.clampDropIndex(toIndex);
        const insertAt = dropIndex > fromIdx ? dropIndex - 1 : dropIndex;
        if (insertAt === fromIdx) {
            return;
        }
        const next = [...this.tabs];
        const [moved] = next.splice(fromIdx, 1);
        next.splice(insertAt, 0, moved);
        this.tabs = next;
        this.notify(EVENT_TYPE.OPEN, this.tabs);
    }

    dropPane(fromPaneId: string, toPaneId: string, side: number): void {
        if (fromPaneId === toPaneId) {
            return;
        }
        const tab = this.tabs.find(t => t.panes.some(p => p.id === fromPaneId) && t.panes.some(p => p.id === toPaneId));
        if (!tab || !tab.paneTree) {
            return;
        }
        const newLeaf: PaneLeaf = { kind: PANE_NODE_KIND.LEAF, id: generateId(), paneId: fromPaneId };
        const treeAfterRemove = removeLeafFromTree(tab.paneTree, fromPaneId);
        if (!treeAfterRemove) {
            return;
        }
        const treeAfterInsert = insertNodeNextTo(treeAfterRemove, toPaneId, newLeaf, side);
        this.tabs = this.tabs.map(t => t.id === tab.id ? { ...t, paneTree: treeAfterInsert, focusedPaneId: fromPaneId } : t);
        const nextTab = this.tabs.find(t => t.id === tab.id);
        if (nextTab && this.focusedPaneListener) {
            this.focusedPaneListener(nextTab);
        }
        this.notify(EVENT_TYPE.OPEN, this.tabs);
    }


    signalDropLanding(rect: DropRect): void {
        this.notify(EVENT_TYPE.DROP_LANDING, rect);
    }

    canDockTabToPane(fromTabId: string, toPaneId: string): boolean {
        const fromTab = this.tabs.find(tab => tab.id === fromTabId);
        const toTab = this.tabs.find(tab => tab.panes.some(pane => pane.id === toPaneId));
        if (!fromTab || !toTab || fromTab.id === toTab.id || !fromTab.paneTree || !toTab.paneTree) {
            return false;
        }
        return this.canTabMergeToPane(fromTab, toTab, toPaneId);
    }

    dockTabToPane(fromTabId: string, toPaneId: string, side: number): void {
        const fromTab = this.tabs.find(t => t.id === fromTabId);
        const toTab = this.tabs.find(t => t.panes.some(p => p.id === toPaneId));
        if (!fromTab || !toTab || fromTab.id === toTab.id || !fromTab.paneTree || !toTab.paneTree) {
            return;
        }
        if (!this.canTabMergeToPane(fromTab, toTab, toPaneId)) {
            return;
        }
        const mergedPanes = [...toTab.panes, ...fromTab.panes];
        let nextFocusedPaneId = fromTab.focusedPaneId;
        if (!nextFocusedPaneId) {
            const firstFrom = fromTab.panes[0];
            if (firstFrom && firstFrom.id) {
                nextFocusedPaneId = firstFrom.id;
            }
        }
        if (!nextFocusedPaneId) {
            const first = mergedPanes[0];
            if (first && first.id) {
                nextFocusedPaneId = first.id;
            }
        }
        const newTree = insertNodeNextTo(toTab.paneTree, toPaneId, fromTab.paneTree, side);
        this.forgetActiveHistory(fromTabId);
        this.tabs = this.tabs
            .filter(t => t.id !== fromTabId)
            .map(t => t.id === toTab.id ? { ...t, panes: mergedPanes, paneTree: newTree, focusedPaneId: nextFocusedPaneId } : t);
        this.activateTabId(toTab.id);
        const nextTab = this.tabs.find(t => t.id === toTab.id);
        if (nextTab && this.focusedPaneListener) {
            this.focusedPaneListener(nextTab);
        }
        this.notify(EVENT_TYPE.OPEN, this.tabs);
    }

    undockPaneToTab(paneId: string, insertAt: number): void {
        const fromTab = this.tabs.find(t => t.panes.some(p => p.id === paneId));
        if (!fromTab || !fromTab.paneTree || fromTab.panes.length <= 1) {
            return;
        }
        const pane = fromTab.panes.find(p => p.id === paneId);
        if (!pane) {
            return;
        }
        const remainingTree = removeLeafFromTree(fromTab.paneTree, paneId);
        if (!remainingTree) {
            return;
        }
        let nextFromFocused = fromTab.focusedPaneId;
        if (nextFromFocused === paneId) {
            const remainingPanes = fromTab.panes.filter(p => p.id !== paneId);
            const first = remainingPanes[0];
            if (first && first.id) {
                nextFromFocused = first.id;
            } else {
                nextFromFocused = undefined;
            }
        }
        const newTab: TabConfig = {
            panes: [pane],
            id: generateId(),
            focusedPaneId: pane.id,
            paneTree: { kind: PANE_NODE_KIND.LEAF, id: generateId(), paneId: pane.id! },
        };
        const updatedFromTab = { ...fromTab, panes: fromTab.panes.filter(p => p.id !== paneId), paneTree: remainingTree, focusedPaneId: nextFromFocused };
        const sourceTabs = this.tabs.map(t => t.id === fromTab.id ? updatedFromTab : t);
        const clampedAt = Math.min(this.clampDropIndex(insertAt), sourceTabs.length);
        const next = [...sourceTabs];
        next.splice(clampedAt, 0, newTab);
        this.tabs = next;
        this.activateTabId(newTab.id);
        const updated = this.tabs.find(t => t.id === updatedFromTab.id);
        if (updated && this.focusedPaneListener) {
            this.focusedPaneListener(updated);
        }
        if (this.focusedPaneListener) {
            this.focusedPaneListener(newTab);
        }
        this.notify(EVENT_TYPE.OPEN, this.tabs);
    }

    private ensureTabAndPaneIds(config: TabConfig): void {
        if (isEmpty(config.id)) {
            config.id = generateId();
        }
        this.ensurePaneIds(config.panes);
        if (!config.focusedPaneId) {
            const first = config.panes[0];
            if (first && first.id) {
                config.focusedPaneId = first.id;
            }
        }
        if (config.paneTree) {
            return
        }
        config.paneTree = buildPaneTree(config.panes);
    }

    private ensurePaneIds(panes: PaneConfig[]): void {
        for (let i = 0; i < panes.length; i += 1) {
            const pane = panes[i];
            if (!isEmpty(pane.id)) {
                continue
            }
            pane.id = generateId();
        }
    }

    setSplitRatio(splitId: string, ratio: number): void {
        let changed = false;
        const nextTabs = this.tabs.map(tab => {
            if (!tab.paneTree) {
                return tab;
            }
            const nextTree = applyRatioToTree(tab.paneTree, splitId, ratio);
            if (nextTree === tab.paneTree) {
                return tab;
            }
            changed = true;
            return { ...tab, paneTree: nextTree };
        });
        if (!changed) {
            return;
        }
        this.tabs = nextTabs;
        this.notifyLayout(TAB_LAYOUT_REASON.SPLIT_RATIO);
    }

    private notifyLayout(reason: number): void {
        const data: TabLayoutEventData = { reason, tabs: this.tabs };
        this.notify(EVENT_TYPE.LAYOUT, data);
    }


    private activateTabId(tabId: string | undefined): void {
        if (this.activeTabId === tabId) {
            this.notify(EVENT_TYPE.ACTIVE, this.activeTabId);
            return;
        }
        const previousId = this.activeTabId;
        if (previousId && this.tabs.some(tab => tab.id === previousId)) {
            this.pushActiveHistory(previousId);
        }
        this.activeTabId = tabId;
        this.notify(EVENT_TYPE.ACTIVE, this.activeTabId);
    }

    private pushActiveHistory(tabId: string | undefined): void {
        if (!tabId) {
            return;
        }
        for (let i = this.activeHistory.length - 1; i >= 0; i -= 1) {
            if (this.activeHistory[i] === tabId) {
                this.activeHistory.splice(i, 1);
            }
        }
        this.activeHistory.push(tabId);
    }

    private forgetActiveHistory(tabId: string): void {
        for (let i = this.activeHistory.length - 1; i >= 0; i -= 1) {
            if (this.activeHistory[i] === tabId) {
                this.activeHistory.splice(i, 1);
            }
        }
    }

    private resolveHistoryActiveId(removedIdx: number): string | undefined {
        while (this.activeHistory.length > 0) {
            const candidate = this.activeHistory.pop();
            if (!candidate) {
                continue;
            }
            if (this.tabs.some(tab => tab.id === candidate)) {
                return candidate;
            }
        }
        return this.resolveFallbackActiveId(removedIdx);
    }

    private resolveFallbackActiveId(removedIdx: number): string | undefined {
        if (this.tabs.length === 0) {
            return undefined;
        }
        if (removedIdx < this.tabs.length) {
            return this.tabs[removedIdx].id;
        }
        return this.tabs[this.tabs.length - 1].id;
    }

    private armTabDragState(state: PaneDragState, pointerId: number, startX: number, startY: number): () => void {
        return armPointerDrag(pointerId, startX, startY, this.TAB_DRAG_START_DISTANCE_PX,
            (clientX, clientY) => this.onTabDragStart(state, clientX, clientY),
            (clientX, clientY) => this.onTabDragMove(state, clientX, clientY),
            (clientX, clientY) => this.onTabDragEnd(state, clientX, clientY),);
    }

    private applyTabDragPos(state: PaneDragState, phaseType: number, clientX: number, clientY: number): void {
        state.phaseType = phaseType;
        state.clientX = clientX;
        state.clientY = clientY;
    }

    private onTabDragStart(state: PaneDragState, clientX: number, clientY: number): void {
        this.applyTabDragPos(state, PHASE_TYPE.START, clientX, clientY);
        this.notify(EVENT_TYPE.DRAG, state);
    }

    private onTabDragMove(state: PaneDragState, clientX: number, clientY: number): void {
        this.applyTabDragPos(state, PHASE_TYPE.MOVE, clientX, clientY);
        this.dragMovePending = true;
        this.scheduleDragMoveNotify(state);
    }

    private onTabDragEnd(state: PaneDragState, clientX: number, clientY: number): void {
        this.clearDragArm();
        this.applyTabDragPos(state, PHASE_TYPE.END, clientX, clientY);
        this.notify(EVENT_TYPE.DRAG, state);
    }

    private clearDragArm(): void {
        if (this.dragDispose) {
            this.dragDispose();
            this.dragDispose = null;
        }
        if (this.dragFrame) {
            cancelAnimationFrame(this.dragFrame);
            this.dragFrame = 0;
        }
        this.dragMovePending = false;
    }

    private scheduleDragMoveNotify(state: PaneDragState): void {
        if (this.dragFrame) {
            return;
        }
        this.dragFrame = requestAnimationFrame(() => {
            this.dragFrame = 0;
            if (!this.dragMovePending) {
                return;
            }
            this.dragMovePending = false;
            this.notify(EVENT_TYPE.DRAG, state);
        });
    }

    private notify(event: number, data: any): void {
        const list = this.listeners;
        for (let i = 0; i < list.length; i += 1) {
            list[i](event, data);
        }
    }
}
