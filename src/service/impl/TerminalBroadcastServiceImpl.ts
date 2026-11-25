import {inject, injectable} from 'inversify';
import {BaseListenerService} from './BaseListenerService';
import {ENTITY_TYPE} from '@/common/const/EntityTypeConst.ts';
import type {TabConfig} from '@/common/components/tab/core/types.ts';
import type {TerminalBroadcastService} from '../TerminalBroadcastService';
import type {BroadcastTerminal} from '@/common/model/dto/BroadcastTerminal.ts';
import type {TabManagerService} from '@/common/components/tab/service/TabManagerService.ts';
import {TabManagerServiceImpl} from '@/common/components/tab/service/impl/TabManagerServiceImpl.ts';

interface Entry {
    label: string;
    input: (data: string) => void;
}

@injectable()
export class TerminalBroadcastServiceImpl extends BaseListenerService implements TerminalBroadcastService {

    private readonly targets = new Set<string>();
    private readonly KEY_CHANGE = 'broadcast-change';
    private readonly entries = new Map<string, Entry>();

    constructor(@inject(TabManagerServiceImpl) private readonly tabManagerService: TabManagerService) {
        super();
    }

    protected getNotifyEntityType(): number {
        return ENTITY_TYPE.NONE;
    }

    register(paneId: string, label: string, input: (data: string) => void): () => void {
        this.entries.set(paneId, {label, input});
        this.notifyListeners<void>(this.KEY_CHANGE, undefined);
        return () => {
            this.entries.delete(paneId);
            this.notifyListeners<void>(this.KEY_CHANGE, undefined);
        };
    }

    listTerminals(): BroadcastTerminal[] {
        return [...this.entries.entries()].map(([paneId, entry]) => ({paneId, label: entry.label}));
    }

    getTargets(tabId: string): string[] {
        if (!this.targets.has(tabId)) {
            return [];
        }
        const tab = this.tabManagerService.getTab(tabId);
        if (!tab) {
            return [];
        }
        const paneIds = this.livePaneIds(tab);
        if (paneIds.length < 2) {
            return [];
        }
        return paneIds;
    }

    setTargets(tabId: string, active: boolean): void {
        if (active) {
            this.targets.add(tabId);
        } else {
            this.targets.delete(tabId);
        }
        this.notifyListeners<void>(this.KEY_CHANGE, undefined);
    }

    isTarget(paneId: string): boolean {
        const tab = this.tabOf(paneId);
        if (!tab) {
            return false;
        }
        if (!tab.id) {
            throw new Error('tab.id unavailable');
        }
        if (!this.targets.has(tab.id)) {
            return false;
        }
        return this.livePaneIds(tab).length > 1;
    }

    dispatch(sourcePaneId: string, data: string): void {
        const tab = this.tabOf(sourcePaneId);
        if (!tab) {
            return;
        }
        if (!tab.id) {
            throw new Error('tab.id unavailable');
        }
        if (!this.targets.has(tab.id)) {
            return;
        }
        const paneIds = this.livePaneIds(tab);
        if (paneIds.length < 2) {
            this.targets.delete(tab.id);
            this.notifyListeners<void>(this.KEY_CHANGE, undefined);
            return;
        }
        for (const paneId of paneIds) {
            if (paneId === sourcePaneId) {
                continue;
            }
            const entry = this.entries.get(paneId);
            if (!entry) {
                continue;
            }
            entry.input(data);
        }
    }

    addChangeListener(cb: () => void): () => void {
        return this.addListener<void>(this.KEY_CHANGE, cb);
    }

    private tabOf(paneId: string): TabConfig | null {
        for (const tab of this.tabManagerService.getTabs()) {
            if (tab.panes.some(pane => pane.id === paneId)) {
                return tab;
            }
        }
        return null;
    }

    private livePaneIds(tab: TabConfig): string[] {
        const paneIds: string[] = [];
        for (const pane of tab.panes) {
            if (!pane.id) {
                throw new Error('pane.id unavailable');
            }
            paneIds.push(pane.id);
        }
        return paneIds;
    }
}
