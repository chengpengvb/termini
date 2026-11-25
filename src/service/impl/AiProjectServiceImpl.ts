import { inject, injectable } from 'inversify';
import { BaseApiServiceImpl } from './BaseApiServiceImpl';
import type { AiProjectService } from '../AiProjectService';
import type { AiProject } from '@/common/model/AiProject.ts';
import { ENTITY_TYPE } from '@/common/const/EntityTypeConst.ts';
import { WebSocketServiceImpl } from '@/service/impl/WebSocketServiceImpl.ts';
import type { RespSocketContext } from '@/common/model/resp/RespWsContext.ts';
import { isAiProjectEvent } from '@/common/model/resp/event/RespEventAiProject.ts';

@injectable()
export class AiProjectServiceImpl extends BaseApiServiceImpl<AiProject> implements AiProjectService {
    private projectList: AiProject[] = [];
    private readonly LISTENER_KEY = 'ai-project-list-change';

    constructor(
        @inject(WebSocketServiceImpl) private readonly webSocketService: WebSocketServiceImpl,
    ) {
        super(webSocketService);
        this.webSocketService.addWsListener((context: RespSocketContext) => {
            if (!isAiProjectEvent(context.type)) {
                return;
            }
            void this.refresh();
        });
    }

    protected getNotifyEntityType(): number {
        return ENTITY_TYPE.AI_PROJECT;
    }

    async start(): Promise<void> {
        void this.refresh();
        return super.start();
    }

    protected override onEntityRefresh(): void {
        void this.refresh();
    }

    protected getPrefix(): string {
        return '/ai_project';
    }

    override async selectAll(): Promise<AiProject[]> {
        const data = await this.request<AiProject[]>(this.buildUrl('/list'));
        if (!data) {
            this.projectList = [];
            return this.projectList;
        }
        this.projectList = data;
        return this.projectList;
    }

    async create(name: string): Promise<AiProject> {
        const data = await this.request<AiProject>(this.buildUrl('/create'), {
            method: 'POST',
            headers: this.JSON_HEADERS,
            body: JSON.stringify({ name }),
        });
        if (!data) {
            throw new Error('create returned no data');
        }
        await this.refresh();
        return data;
    }

    async rename(id: string, name: string): Promise<void> {
        await this.request<unknown>(this.buildUrl('/rename'), {
            method: 'POST',
            headers: this.JSON_HEADERS,
            body: JSON.stringify({ id, name }),
        });
        await this.refresh();
    }

    override async deleteById(id: string): Promise<void> {
        await super.deleteById(id);
        await this.refresh();
    }

    addAiProjectListChangeListener(listener: (list: AiProject[]) => void): () => void {
        return this.addListener<AiProject[]>(this.LISTENER_KEY, listener);
    }

    private async refresh(): Promise<void> {
        await this.selectAll();
        this.notifyListeners<AiProject[]>(this.LISTENER_KEY, this.projectList);
    }
}
