import {inject, injectable} from 'inversify';
import {BaseApiServiceImpl} from './BaseApiServiceImpl';
import {ENTITY_TYPE} from '@/common/const/EntityTypeConst.ts';
import type {JobDiscoverService} from '../JobDiscoverService';
import {WebSocketServiceImpl} from '@/service/impl/WebSocketServiceImpl.ts';
import type {RespSocketContext} from '@/common/model/resp/RespWsContext.ts';
import type {JobDiscover, JobDiscoverItem} from '@/common/model/JobDiscover.ts';
import {isJobDiscoverEvent} from '@/common/model/resp/event/RespEventJobDiscover.ts';
import {JOB_DISCOVER_DRAWER_TAB, type JobDiscoverDrawerTab} from '@/page/common/model/JobModel';

@injectable()
export class JobDiscoverServiceImpl extends BaseApiServiceImpl<JobDiscover> implements JobDiscoverService {
    private discoverList: JobDiscover[] = [];
    private drawerTab: JobDiscoverDrawerTab = JOB_DISCOVER_DRAWER_TAB.RESULT;
    private readonly LISTENER_KEY = 'job-discover-list-change';
    private readonly STORAGE_KEY_DRAWER_TAB = 'argus:job:discover-drawer-tab';

    constructor(
        @inject(WebSocketServiceImpl) private readonly webSocketService: WebSocketServiceImpl,
    ) {
        super(webSocketService);
        this.drawerTab = this.loadDrawerTab();
        this.webSocketService.addWsListener((context: RespSocketContext) => {
            if (!isJobDiscoverEvent(context.type)) {
                return;
            }
            void this.refresh();
        });
    }

    protected getNotifyEntityType(): number {
        return ENTITY_TYPE.JOB_DISCOVER;
    }

    protected override onEntityRefresh(): void {
        void this.refresh();
    }

    protected getPrefix(): string {
        return '/job_discover';
    }

    addJobDiscoverListChangeListener(listener: (list: JobDiscover[]) => void): () => void {
        return this.addListener<JobDiscover[]>(this.LISTENER_KEY, listener);
    }

    getDrawerTab(): JobDiscoverDrawerTab {
        return this.drawerTab;
    }

    setDrawerTab(tab: JobDiscoverDrawerTab): void {
        if (tab !== JOB_DISCOVER_DRAWER_TAB.RESULT && tab !== JOB_DISCOVER_DRAWER_TAB.ECHO) {
            return;
        }
        this.drawerTab = tab;
        window.localStorage.setItem(this.STORAGE_KEY_DRAWER_TAB, String(tab));
    }

    override async selectAll(): Promise<JobDiscover[]> {
        const data = await this.request<JobDiscover[]>(this.buildUrl('/select_all'));
        this.discoverList = (data ?? []).slice().sort((left, right) => right.timestamp - left.timestamp);
        return this.discoverList;
    }

    async handleStart(jobDeviceId: string): Promise<JobDiscover | null> {
        const started = await this.request<JobDiscover>(this.buildUrl(`/start/${jobDeviceId}`), {method: 'POST'});
        await this.refresh();
        return started;
    }

    async handleCancel(chatId: string): Promise<void> {
        await this.request<unknown>(this.buildUrl(`/cancel/${chatId}`), {method: 'POST'});
    }

    async selectItem(discoverId: string): Promise<JobDiscoverItem[]> {
        const data = await this.request<JobDiscoverItem[]>(this.buildUrl(`/select_item/${discoverId}`));
        return data ?? [];
    }

    async accept(ids: string[]): Promise<void> {
        await this.request<unknown>(this.buildUrl('/accept'), {
            method: 'POST',
            headers: this.JSON_HEADERS,
            body: JSON.stringify(ids),
        });
    }

    async deleteBatch(ids: string[]): Promise<number> {
        if (ids.length === 0) {
            return 0;
        }
        const data = await this.request<number>(this.buildUrl('/delete_batch'), {
            method: 'POST',
            headers: this.JSON_HEADERS,
            body: JSON.stringify(ids),
        });
        await this.refresh();
        return data ?? 0;
    }

    private async refresh(): Promise<void> {
        await this.selectAll();
        this.notifyListeners<JobDiscover[]>(this.LISTENER_KEY, this.discoverList);
    }

    private loadDrawerTab(): JobDiscoverDrawerTab {
        const raw = window.localStorage.getItem(this.STORAGE_KEY_DRAWER_TAB);
        if (!raw) {
            return JOB_DISCOVER_DRAWER_TAB.RESULT;
        }
        const parsed = Number(raw);
        if (parsed === JOB_DISCOVER_DRAWER_TAB.RESULT || parsed === JOB_DISCOVER_DRAWER_TAB.ECHO) {
            return parsed;
        }
        return JOB_DISCOVER_DRAWER_TAB.RESULT;
    }
}
