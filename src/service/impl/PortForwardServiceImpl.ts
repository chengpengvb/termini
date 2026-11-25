import {inject, injectable} from 'inversify';
import {isNotEmpty} from '@/common/util/StringUtil.ts';
import {BaseApiServiceImpl} from './BaseApiServiceImpl';
import {ENTITY_TYPE} from '@/common/const/EntityTypeConst.ts';
import type {PortForwardService} from '../PortForwardService';
import {AccountServiceImpl} from '@/service/impl/AccountServiceImpl.ts';
import {FORWARD_STATUS, type PortForward} from '@/common/model/PortForward';
import {WebSocketServiceImpl} from '@/service/impl/WebSocketServiceImpl.ts';
import type {RespSocketContext} from '@/common/model/resp/RespWsContext.ts';
import {isForwardEvent, RESP_EVENT_FORWARD} from '@/common/model/resp/event/RespEventForward.ts';

@injectable()
export class PortForwardServiceImpl extends BaseApiServiceImpl<PortForward> implements PortForwardService {
    private forwardList: PortForward[] = [];
    private readonly LISTENER_KEY = 'port-forward-list-change';

    constructor(
        @inject(AccountServiceImpl) private readonly accountService: AccountServiceImpl,
        @inject(WebSocketServiceImpl) private readonly webSocketService: WebSocketServiceImpl,
    ) {
        super(webSocketService);
        this.webSocketService.addWsListener((context: RespSocketContext) => {
            this.handleRespContext(context);
        });
        this.accountService.addCloudSyncDownloadedListener(() => {
            void this.reloadAll();
        });
    }

    protected getNotifyEntityType(): number {
        return ENTITY_TYPE.PORT_FORWARD;
    }

    async start(): Promise<void> {
        void this.reloadAll();
        return super.start();
    }

    protected override onEntityRefresh(): void {
        void this.reloadAll();
    }

    protected getPrefix(): string {
        return '/port_forward';
    }

    addForwardListChangeListener(listener: (list: PortForward[]) => void): () => void {
        return this.addListener<PortForward[]>(this.LISTENER_KEY, listener);
    }

    async reloadAll(): Promise<void> {
        const data = await this.request<PortForward[]>(this.buildUrl('/select_all'));
        this.forwardList = data ?? [];
        this.notifyListeners<PortForward[]>(this.LISTENER_KEY, this.forwardList);
    }

    async selectByDevice(deviceId: string): Promise<PortForward[]> {
        const data = await this.request<PortForward[]>(this.buildUrl(`/select_by_device/${deviceId}`));
        if (!data) {
            return [];
        }
        return data;
    }

    override async insert(item: PortForward): Promise<PortForward> {
        const saved = await super.insert(item);
        await this.reloadAll();
        return saved;
    }

    override async update(item: PortForward): Promise<PortForward> {
        const saved = await super.update(item);
        await this.reloadAll();
        return saved;
    }

    override async deleteById(id: string): Promise<void> {
        await super.deleteById(id);
        await this.reloadAll();
    }

    private handleRespContext(context: RespSocketContext): void {
        if (!isForwardEvent(context.type)) {
            return;
        }
        const forwardId = context.session_id;
        if (!isNotEmpty(forwardId)) {
            return;
        }
        if (context.type === RESP_EVENT_FORWARD.ON_STARTED) {
            this.patchStatus(forwardId, FORWARD_STATUS.RUNNING);
            return;
        }
        if (context.type === RESP_EVENT_FORWARD.ON_STOPPED) {
            this.patchStatus(forwardId, FORWARD_STATUS.IDLE);
            return;
        }
        if (context.type === RESP_EVENT_FORWARD.ON_ERROR) {
            this.patchStatus(forwardId, FORWARD_STATUS.ERROR);
        }
    }

    private patchStatus(forwardId: string, status: number): void {
        let changed = false;
        this.forwardList = this.forwardList.map(item => {
            if (item.id !== forwardId) {
                return item;
            }
            if (item.status === status) {
                return item;
            }
            changed = true;
            return {...item, status};
        });
        if (!changed) {
            return;
        }
        this.notifyListeners<PortForward[]>(this.LISTENER_KEY, this.forwardList);
    }
}
