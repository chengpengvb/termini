import {inject, injectable} from 'inversify';
import {isNil} from '@/common/util/StringUtil.ts';
import {BaseApiServiceImpl} from './BaseApiServiceImpl';
import type {AiProvideService} from '../AiProvideService';
import {ENTITY_TYPE} from '@/common/const/EntityTypeConst.ts';
import type {RespProvider} from '@/common/model/resp/RespProvider.ts';
import {WebSocketServiceImpl} from '@/service/impl/WebSocketServiceImpl.ts';
import {AI_PROVIDE_SOURCE, type AiProvide, type SelectedProvide} from '@/common/model/AiProvide.ts';

@injectable()
export class AiProvideServiceImpl extends BaseApiServiceImpl<AiProvide> implements AiProvideService {
    private aiProvideList: AiProvide[] = [];
    private refreshPromise: Promise<AiProvide[]> | null = null;
    private readonly LISTENER_KEY = 'ai-provide-list-change';
    private readonly STORAGE_KEY_SELECTED_PROVIDE = 'argus:agent:selected-provide';
    private readonly STORAGE_KEY_THINKING_LEVEL = 'argus:agent:thinking-level'; // { [provideId]: level } 按模型记住挡位

    constructor(
        @inject(WebSocketServiceImpl) webSocketService: WebSocketServiceImpl,
    ) {
        super(webSocketService);
    }

    protected getNotifyEntityType(): number {
        return ENTITY_TYPE.AI_PROVIDE;
    }

    protected override onEntityRefresh(): void {
        void this.selectAll().then(() => this.notify());
    }

    protected getPrefix(): string {
        return '/ai_provide';
    }

    addAiProvideListChangeListener(listener: (list: AiProvide[]) => void): () => void {
        return this.addListener<AiProvide[]>(this.LISTENER_KEY, listener);
    }

    getSelectedProvide(): SelectedProvide | null {
        const raw = window.localStorage.getItem(this.STORAGE_KEY_SELECTED_PROVIDE);
        if (!raw || raw.trim().length === 0) {
            return null;
        }
        const parsed = JSON.parse(raw) as SelectedProvide;
        if (!parsed.id || parsed.id.trim().length === 0) {
            return null;
        }
        return parsed;
    }

    setSelectedProvide(id: string | null, source: number): void {
        if (!id || id.trim().length === 0) {
            window.localStorage.removeItem(this.STORAGE_KEY_SELECTED_PROVIDE);
            return;
        }
        window.localStorage.setItem(this.STORAGE_KEY_SELECTED_PROVIDE, JSON.stringify({id, source} satisfies SelectedProvide));
    }

    getThinkingLevel(provideId: string): string | null {
        const level = this.readThinkingLevels()[provideId];
        if (isNil(level) || level.length === 0) {
            return null;
        }
        return level;
    }

    setThinkingLevel(provideId: string, level: string | null): void {
        const map = this.readThinkingLevels();
        if (level === null || level.length === 0) {
            delete map[provideId];
        } else {
            map[provideId] = level;
        }
        window.localStorage.setItem(this.STORAGE_KEY_THINKING_LEVEL, JSON.stringify(map));
    }

    private readThinkingLevels(): Record<string, string> {
        const raw = window.localStorage.getItem(this.STORAGE_KEY_THINKING_LEVEL);
        if (!raw || raw.trim().length === 0) {
            return {};
        }
        const parsed = JSON.parse(raw) as Record<string, string>;
        if (parsed === null) {
            return {};
        }
        return parsed;
    }

    async refresh(): Promise<AiProvide[]> {
        if (this.refreshPromise !== null) {
            return this.refreshPromise;
        }
        const pending = this.loadRefreshedList();
        this.refreshPromise = pending;
        try {
            return await pending;
        } finally {
            if (this.refreshPromise === pending) {
                this.refreshPromise = null;
            }
        }
    }

    private async loadRefreshedList(): Promise<AiProvide[]> {
        const data = await this.request<AiProvide[]>(this.buildUrl('/refresh'));
        if (!data) {
            console.warn('[ai-provide] refresh cloud models data is empty');
            return this.aiProvideList;
        }
        this.aiProvideList = data;
        this.notify();
        return this.aiProvideList;
    }

    async listTemplateProvider(): Promise<RespProvider[]> {
        const data = await this.request<RespProvider[]>(this.buildUrl('/list_template_provider_info'));
        if (!data) {
            return [];
        }
        return data;
    }

    async getTemplateProvider(provideId: number): Promise<RespProvider | null> {
        return this.request<RespProvider>(this.buildUrl(`/get_template_provider_info/${provideId}`));
    }

    override async selectAll(): Promise<AiProvide[]> {
        const data = await this.request<AiProvide[]>(this.buildUrl('/select_all'));
        if (!data) {
            this.aiProvideList = [];
            return this.aiProvideList;
        }
        this.aiProvideList = data;
        return this.aiProvideList;
    }

    override async insert(item: AiProvide): Promise<AiProvide> {
        const saved = await super.insert(item);
        await this.selectAll();
        this.notify();
        return saved;
    }

    override async update(item: AiProvide): Promise<AiProvide> {
        const saved = await super.update(item);
        await this.selectAll();
        this.notify();
        return saved;
    }

    override async deleteById(id: string): Promise<void> {
        await super.deleteById(id);
        const selected = this.getSelectedProvide();
        if (selected && selected.source === AI_PROVIDE_SOURCE.CUSTOM && selected.id === id) {
            this.setSelectedProvide(null, AI_PROVIDE_SOURCE.CUSTOM);
        }
        await this.selectAll();
        this.notify();
    }

    private notify(): void {
        this.notifyListeners<AiProvide[]>(this.LISTENER_KEY, this.aiProvideList);
    }
}
