import {inject, injectable} from 'inversify';
import {BaseApiServiceImpl} from './BaseApiServiceImpl';
import {ENTITY_TYPE} from '@/common/const/EntityTypeConst.ts';
import type {AiMcpServerService} from '../AiMcpServerService';
import {WebSocketServiceImpl} from '@/service/impl/WebSocketServiceImpl.ts';
import type {AiMcpServer, McpMarketItem, McpPromptSnapshot, McpToolSnapshot, ReqMcpMarketInstall, RespAiMcpServerRuntime, RespAiMcpServerTools} from '@/common/model/AiMcpServer.ts';

@injectable()
export class AiMcpServerServiceImpl extends BaseApiServiceImpl<AiMcpServer> implements AiMcpServerService {
    constructor(
        @inject(WebSocketServiceImpl) webSocketService: WebSocketServiceImpl,
    ) {
        super(webSocketService);
    }

    protected getNotifyEntityType(): number {
        return ENTITY_TYPE.AI_MCP_SERVER;
    }

    protected getPrefix(): string {
        return '/ai_mcp_server';
    }

    async listRuntime(): Promise<RespAiMcpServerRuntime[]> {
        const data = await this.request<RespAiMcpServerRuntime[]>(this.buildUrl('/list_runtime'));
        if (!data) {
            return [];
        }
        return data;
    }

    async probe(server: AiMcpServer): Promise<McpToolSnapshot[]> {
        const data = await this.request<McpToolSnapshot[]>(this.buildUrl('/probe'), {
            method: 'POST',
            headers: this.JSON_HEADERS,
            body: JSON.stringify(server),
        });
        if (!data) {
            return [];
        }
        return data;
    }

    async reconnect(id: string): Promise<RespAiMcpServerRuntime> {
        const data = await this.request<RespAiMcpServerRuntime>(this.buildUrl(`/${id}/reconnect`), {
            method: 'POST',
        });
        if (!data) {
            throw new Error('reconnect returned no data');
        }
        return data;
    }

    async listTools(id: string): Promise<RespAiMcpServerTools> {
        const data = await this.request<RespAiMcpServerTools>(this.buildUrl(`/${id}/tools`));
        if (!data) {
            throw new Error('listTools returned no data');
        }
        return data;
    }

    async listConnectedTools(): Promise<McpToolSnapshot[]> {
        const data = await this.request<McpToolSnapshot[]>(this.buildUrl('/tools'));
        if (!data) {
            return [];
        }
        return data;
    }

    async listPrompts(): Promise<McpPromptSnapshot[]> {
        const data = await this.request<McpPromptSnapshot[]>(this.buildUrl('/prompts'));
        if (!data) {
            return [];
        }
        return data;
    }

    async listMarket(): Promise<McpMarketItem[]> {
        const data = await this.request<McpMarketItem[]>(this.buildUrl('/market/list'));
        if (!data) {
            return [];
        }
        return data;
    }

    async installMarket(req: ReqMcpMarketInstall): Promise<AiMcpServer> {
        const data = await this.request<AiMcpServer>(this.buildUrl('/market/install'), {
            method: 'POST',
            headers: this.JSON_HEADERS,
            body: JSON.stringify(req),
        });
        if (!data) {
            throw new Error('installMarket returned no data');
        }
        return data;
    }
}
