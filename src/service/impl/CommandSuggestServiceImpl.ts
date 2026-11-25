import {inject, injectable} from 'inversify';
import {API_BASE_URL} from '@/runtime-config.ts';
import {BaseApiServiceImpl} from './BaseApiServiceImpl';
import {ENTITY_TYPE} from '@/common/const/EntityTypeConst.ts';
import type {CommandSuggestService} from '../CommandSuggestService';
import {WebSocketServiceImpl} from '@/service/impl/WebSocketServiceImpl.ts';
import type {ReqCommandSuggest} from '@/common/model/req/ReqCommandSuggest';

@injectable()
export class CommandSuggestServiceImpl extends BaseApiServiceImpl<never> implements CommandSuggestService {

    private readonly sshHistoryCache: Map<string, string[]> = new Map();

    constructor(
        @inject(WebSocketServiceImpl) webSocketService: WebSocketServiceImpl,
    ) {
        super(webSocketService);
    }

    protected getNotifyEntityType(): number {
        return ENTITY_TYPE.NONE;
    }

    protected getPrefix(): string {
        return '/command_suggest';
    }

    async suggest(req: ReqCommandSuggest, signal: AbortSignal, onPartial: (list: string[]) => void): Promise<string[]> {
        return this.requestNdjson<string[]>(
            this.buildUrl('/suggest'),
            {
                signal,
                method: 'POST',
                headers: this.JSON_HEADERS,
                body: JSON.stringify(req),
            },
            onPartial,
        );
    }

    async loadSshHistory(deviceId: string): Promise<string[]> {
        const cached = this.sshHistoryCache.get(deviceId);
        if (cached && cached.length > 0) {
            return cached;
        }
        const list = await this.request<string[]>(`${API_BASE_URL}/ssh/cmd/history/${deviceId}`);
        const history = list ?? [];
        if (history.length > 0) {
            this.sshHistoryCache.set(deviceId, history);
        }
        return history;
    }

    async loadLocalHistory(shellId: string): Promise<string[]> {
        const list = await this.request<string[]>(`${API_BASE_URL}/shell/cmd/history/${encodeURIComponent(shellId)}`);
        return list ?? [];
    }
}
