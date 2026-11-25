import {inject, injectable} from 'inversify';
import {Device} from "@/common/model/Device.ts";
import type {DeviceService} from '../DeviceService';
import {BaseApiServiceImpl} from './BaseApiServiceImpl';
import {ReqDevice} from "@/common/model/req/ReqDevice.ts";
import {isEmpty, isNil} from "@/common/util/StringUtil.ts";
import {ENTITY_TYPE} from '@/common/const/EntityTypeConst.ts';
import type {RespTree} from '@/common/model/resp/RespTree.ts';
import {CHAT_TARGET_TYPE} from '@/common/const/ChatTargetConst.ts';
import {AccountServiceImpl} from '@/service/impl/AccountServiceImpl.ts';
import {WebSocketServiceImpl} from '@/service/impl/WebSocketServiceImpl.ts';
import {HistoryLoginServiceImpl} from '@/service/impl/HistoryLoginServiceImpl.ts';

@injectable()
export class DeviceServiceImpl extends BaseApiServiceImpl<Device> implements DeviceService {

    private deviceCacheReady = false;
    private reqDeviceList: ReqDevice[] = [];
    private readonly deviceById = new Map<string, Device>();
    private readonly LISTENER_KEY_DEVICE_LIST_CHANGE = 'device-list-change';

    constructor(
        @inject(WebSocketServiceImpl) webSocketService: WebSocketServiceImpl,
        @inject(AccountServiceImpl) private readonly accountService: AccountServiceImpl,
        @inject(HistoryLoginServiceImpl) private readonly historyLoginService: HistoryLoginServiceImpl,
    ) {
        super(webSocketService);
        this.accountService.addCloudSyncDownloadedListener(() => {
            void this.selectAllWithConfig();
        });
    }

    protected getNotifyEntityType(): number {
        return ENTITY_TYPE.DEVICE;
    }

    async start(): Promise<void> {
        void this.selectAllWithConfig();
        return super.start();
    }

    protected override onEntityRefresh(): void {
        void this.selectAllWithConfig();
    }

    protected getPrefix(): string {
        return '/device';
    }

    addDeviceListChangeListener(listener: (list: ReqDevice[]) => void): () => void {
        return this.addListener<ReqDevice[]>(this.LISTENER_KEY_DEVICE_LIST_CHANGE, listener);
    }

    getCachedDevice(id: string): Device | null {
        if (!id) {
            return null;
        }
        const device = this.deviceById.get(id);
        if (isNil(device)) {
            return null;
        }
        return device;
    }

    resolveDeviceLabel(id: string): string | null {
        const device = this.getCachedDevice(id);
        if (isNil(device)) {
            return null;
        }
        if (!device.name) {
            return null;
        }
        if (!device.host) {
            return null;
        }
        return `${device.name}(${device.host})`;
    }

    async ensureDeviceCache(): Promise<void> {
        if (this.deviceCacheReady) {
            return;
        }
        await this.selectAllWithConfig();
    }

    async saveOrUpdateWithConfig(req: ReqDevice): Promise<ReqDevice | null> {
        const actionPath = isEmpty(req.device.id) ? '/save_with_config' : '/update_with_config';
        const saved = await this.request<ReqDevice>(this.buildUrl(actionPath), {
            method: 'POST',
            headers: this.JSON_HEADERS,
            body: JSON.stringify(req),
        });
        await this.selectAllWithConfig();
        if (!saved) {
            console.warn('[device] save_with_config returned no data');
            return null;
        }
        return saved;
    }

    async selectWithConfig(id: string | undefined): Promise<ReqDevice | null> {
        if (!id) {
            return null;
        }
        return this.request<ReqDevice>(this.buildUrl(`/${id}/with_config`));
    }

    async localDevice(): Promise<ReqDevice | null> {
        const data = await this.request<ReqDevice>(this.buildUrl('/local_device'));
        if (!data) {
            console.warn('[device] local_device returned no data');
            return null;
        }
        return data;
    }

    async delete(id: string | undefined): Promise<void> {
        if (!id) {
            return;
        }
        await this.request<unknown>(this.buildUrl(`/${id}/delete_with_config`));
        this.historyLoginService.removeByDeviceId(id);
        await this.selectAllWithConfig();
    }

    async selectAllWithConfig(): Promise<void> {
        const data = await this.request<ReqDevice[]>(this.buildUrl('/select_all_with_config'));
        if (!data) {
            console.warn('[device] select_all_with_config returned no data');
            return;
        }
        this.reqDeviceList = data;
        this.replaceDeviceMap(data.map((item) => item.device));
        await this.notify();
    }

    async selectDeviceTree(): Promise<RespTree[]> {
        const data = await this.request<RespTree[]>(this.buildUrl('/select_device_tree'));
        if (!data) {
            return [];
        }
        this.replaceDeviceMap(this.collectDevicesFromTree(data));
        return data;
    }

    private collectDevicesFromTree(nodes: RespTree[]): Device[] {
        const devices: Device[] = [];
        for (const node of nodes) {
            if (node.type === CHAT_TARGET_TYPE.DEVICE && node.data) {
                devices.push(JSON.parse(node.data) as Device);
            }
            if (!node.children) {
                continue;
            }
            for (const child of this.collectDevicesFromTree(node.children)) {
                devices.push(child);
            }
        }
        return devices;
    }

    private replaceDeviceMap(devices: Device[]): void {
        this.deviceById.clear();
        for (const device of devices) {
            if (!device.id) {
                continue;
            }
            this.deviceById.set(device.id, device);
        }
        this.deviceCacheReady = true;
    }

    private async notify(): Promise<void> {
        this.notifyListeners<ReqDevice[]>(this.LISTENER_KEY_DEVICE_LIST_CHANGE, this.reqDeviceList);
    }
}
