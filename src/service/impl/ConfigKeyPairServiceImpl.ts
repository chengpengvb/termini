import {inject, injectable} from 'inversify';
import {BaseApiServiceImpl} from './BaseApiServiceImpl';
import {ENTITY_TYPE} from '@/common/const/EntityTypeConst.ts';
import type {ConfigKeyPair} from '@/common/model/ConfigKeyPair';
import type {ConfigKeyPairService} from '../ConfigKeyPairService';
import {AccountServiceImpl} from '@/service/impl/AccountServiceImpl.ts';
import {WebSocketServiceImpl} from '@/service/impl/WebSocketServiceImpl.ts';

@injectable()
export class ConfigKeyPairServiceImpl extends BaseApiServiceImpl<ConfigKeyPair> implements ConfigKeyPairService {
    private keyPairList: ConfigKeyPair[] = [];
    private readonly LISTENER_KEY = 'key-pair-list-change';

    constructor(
        @inject(WebSocketServiceImpl) webSocketService: WebSocketServiceImpl,
        @inject(AccountServiceImpl) private readonly accountService: AccountServiceImpl,
    ) {
        super(webSocketService);
        this.accountService.addCloudSyncDownloadedListener(() => {
            void this.reloadAll();
        });
    }

    protected getNotifyEntityType(): number {
        return ENTITY_TYPE.CONFIG_KEY_PAIR;
    }

    async start(): Promise<void> {
        void this.reloadAll();
        return super.start();
    }

    protected override onEntityRefresh(): void {
        void this.reloadAll();
    }

    protected getPrefix(): string {
        return '/config_key_pair';
    }

    addKeyPairListChangeListener(listener: (list: ConfigKeyPair[]) => void): () => void {
        return this.addListener<ConfigKeyPair[]>(this.LISTENER_KEY, listener);
    }

    async reloadAll(): Promise<void> {
        const data = await this.request<ConfigKeyPair[]>(this.buildUrl('/select_all'));
        this.keyPairList = data ?? [];
        this.notifyListeners<ConfigKeyPair[]>(this.LISTENER_KEY, this.keyPairList);
    }

    override async insert(item: ConfigKeyPair): Promise<ConfigKeyPair> {
        const saved = await super.insert(item);
        await this.reloadAll();
        return saved;
    }

    override async update(item: ConfigKeyPair): Promise<ConfigKeyPair> {
        const saved = await super.update(item);
        await this.reloadAll();
        return saved;
    }

    override async deleteById(id: string): Promise<void> {
        await super.deleteById(id);
        await this.reloadAll();
    }
}
