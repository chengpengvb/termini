import {inject, injectable} from 'inversify';
import {BaseApiServiceImpl} from './BaseApiServiceImpl';
import type {JobDeviceService} from '../JobDeviceService';
import type {JobDevice} from '@/common/model/JobDevice.ts';
import {ENTITY_TYPE} from '@/common/const/EntityTypeConst.ts';
import type {RespMetricItem} from '@/common/model/resp/RespMetricItem.ts';
import {WebSocketServiceImpl} from '@/service/impl/WebSocketServiceImpl.ts';
import type {RespSocketContext} from '@/common/model/resp/RespWsContext.ts';
import {isJobMetricEvent} from '@/common/model/resp/event/RespEventJobMetric.ts';
import {JOB_DEVICE_DRAWER_TAB, JOB_MONITOR_VIEW, type JobDeviceDrawerTab, type JobMonitorView} from '@/page/common/model/JobModel';

@injectable()
export class JobDeviceServiceImpl extends BaseApiServiceImpl<JobDevice> implements JobDeviceService {

    private livePins: string[] = [];
    private expandedMap: Map<string, boolean> = new Map();
    private monitorView: JobMonitorView = JOB_MONITOR_VIEW.LIST;
    private drawerTab: JobDeviceDrawerTab = JOB_DEVICE_DRAWER_TAB.DETAIL;
    private readonly LISTENER_KEY_METRIC = 'job-metric-change:';
    private readonly STORAGE_KEY_MONITOR_VIEW = 'argus:job:monitor-view';
    private readonly STORAGE_KEY_DRAWER_TAB = 'argus:job:device-drawer-tab';
    private readonly STORAGE_KEY_EXPANDED = 'argus:job:device-expanded';
    private readonly STORAGE_KEY_LIVE_PINS = 'argus:job:overview-live-pins';

    constructor(
        @inject(WebSocketServiceImpl) private readonly webSocketService: WebSocketServiceImpl,
    ) {
        super(webSocketService);
        this.livePins = this.loadLivePins();
        this.monitorView = this.loadMonitorView();
        this.drawerTab = this.loadDrawerTab();
        this.expandedMap = this.loadExpandedMap();
        this.webSocketService.addWsListener((context: RespSocketContext) => {
            if (!isJobMetricEvent(context.type)) {
                return;
            }
            const metric = context.payload as RespMetricItem | null;
            if (!metric) {
                return;
            }
            this.notifyListeners<RespMetricItem>(this.LISTENER_KEY_METRIC + metric.job_device_id, metric);
        });
    }

    protected getNotifyEntityType(): number {
        return ENTITY_TYPE.JOB_DEVICE;
    }

    addMetricListener(jobDeviceId: string, listener: (metric: RespMetricItem) => void): () => void {
        return this.addListener<RespMetricItem>(this.LISTENER_KEY_METRIC + jobDeviceId, listener);
    }

    protected getPrefix(): string {
        return '/job_device';
    }

    async selectAllDetail(): Promise<JobDevice[]> {
        const data = await this.request<JobDevice[]>(this.buildUrl('/select_all_detail'));
        if (!data) {
            return [];
        }
        return data;
    }

    async takeCollectGap(): Promise<number> {
        const data = await this.request<number>(this.buildUrl('/take_collect_gap'));
        if (!data) {
            return 0;
        }
        return data;
    }

    async selectMetric(jobDeviceId: string, jobIndicatorIds: string[]): Promise<RespMetricItem[]> {
        const data = await this.request<RespMetricItem[]>(this.buildUrl(`/select_metric/${jobDeviceId}`), {
            method: 'POST',
            headers: this.JSON_HEADERS,
            body: JSON.stringify(jobIndicatorIds),
        });
        if (!data) {
            return [];
        }
        return data;
    }

    async delete(id: string): Promise<number> {
        const data = await this.request<number>(this.buildUrl(`/delete/${id}`), {method: 'DELETE'});
        if (!data) {
            return 0;
        }
        return data;
    }

    async saveBatch(deviceIds: string[]): Promise<number> {
        const data = await this.request<number>(this.buildUrl('/save_batch'), {
            method: 'POST',
            headers: this.JSON_HEADERS,
            body: JSON.stringify(deviceIds),
        });
        return data ?? 0;
    }

    async setEnabled(id: string, enabled: boolean): Promise<boolean> {
        const data = await this.request<boolean>(this.buildUrl(`/set_enabled/${id}`), {
            method: 'POST',
            headers: this.JSON_HEADERS,
            body: JSON.stringify(enabled),
        });
        return data ?? enabled;
    }

    getMonitorView(): JobMonitorView {
        return this.monitorView;
    }

    setMonitorView(view: JobMonitorView): void {
        if (view !== JOB_MONITOR_VIEW.LIST && view !== JOB_MONITOR_VIEW.GRID) {
            return;
        }
        this.monitorView = view;
        window.localStorage.setItem(this.STORAGE_KEY_MONITOR_VIEW, String(view));
    }

    getDrawerTab(): JobDeviceDrawerTab {
        return this.drawerTab;
    }

    setDrawerTab(tab: JobDeviceDrawerTab): void {
        if (tab !== JOB_DEVICE_DRAWER_TAB.DETAIL && tab !== JOB_DEVICE_DRAWER_TAB.ALERT) {
            return;
        }
        this.drawerTab = tab;
        window.localStorage.setItem(this.STORAGE_KEY_DRAWER_TAB, String(tab));
    }

    private loadMonitorView(): JobMonitorView {
        const raw = window.localStorage.getItem(this.STORAGE_KEY_MONITOR_VIEW);
        if (!raw) {
            return JOB_MONITOR_VIEW.LIST;
        }
        const parsed = Number(raw);
        if (parsed === JOB_MONITOR_VIEW.LIST || parsed === JOB_MONITOR_VIEW.GRID) {
            return parsed;
        }
        return JOB_MONITOR_VIEW.LIST;
    }

    getLivePins(): string[] {
        return [...this.livePins];
    }

    toggleLivePin(key: string): string[] {
        if (this.livePins.includes(key)) {
            this.livePins = this.livePins.filter(item => item !== key);
        } else {
            this.livePins = [...this.livePins, key];
        }
        window.localStorage.setItem(this.STORAGE_KEY_LIVE_PINS, JSON.stringify(this.livePins));
        return [...this.livePins];
    }

    private loadLivePins(): string[] {
        const raw = window.localStorage.getItem(this.STORAGE_KEY_LIVE_PINS);
        if (!raw) {
            return [];
        }
        try {
            return JSON.parse(raw) as string[];
        } catch {
            return [];
        }
    }

    getExpanded(id: string): boolean {
        const stored = this.expandedMap.get(id);
        return stored !== undefined ? stored : true;
    }

    setExpanded(id: string, expanded: boolean): void {
        this.expandedMap.set(id, expanded);
        const obj: Record<string, boolean> = {};
        this.expandedMap.forEach((v, k) => { obj[k] = v; });
        window.localStorage.setItem(this.STORAGE_KEY_EXPANDED, JSON.stringify(obj));
    }

    private loadExpandedMap(): Map<string, boolean> {
        const raw = window.localStorage.getItem(this.STORAGE_KEY_EXPANDED);
        if (!raw) {
            return new Map();
        }
        try {
            const obj = JSON.parse(raw) as Record<string, boolean>;
            return new Map(Object.entries(obj));
        } catch {
            return new Map();
        }
    }

    private loadDrawerTab(): JobDeviceDrawerTab {
        const raw = window.localStorage.getItem(this.STORAGE_KEY_DRAWER_TAB);
        if (!raw) {
            return JOB_DEVICE_DRAWER_TAB.DETAIL;
        }
        const parsed = Number(raw);
        if (parsed === JOB_DEVICE_DRAWER_TAB.DETAIL || parsed === JOB_DEVICE_DRAWER_TAB.ALERT) {
            return parsed;
        }
        return JOB_DEVICE_DRAWER_TAB.DETAIL;
    }
}
