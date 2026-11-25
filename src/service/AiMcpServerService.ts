import type {ApiService} from './ApiService';
import type {AiMcpServer, McpMarketItem, McpPromptSnapshot, McpToolSnapshot, ReqMcpMarketInstall, RespAiMcpServerRuntime, RespAiMcpServerTools} from '@/common/model/AiMcpServer.ts';

export interface AiMcpServerService extends ApiService<AiMcpServer> {
    listMarket(): Promise<McpMarketItem[]>;
    listPrompts(): Promise<McpPromptSnapshot[]>;
    listRuntime(): Promise<RespAiMcpServerRuntime[]>;
    listTools(id: string): Promise<RespAiMcpServerTools>;
    listConnectedTools(): Promise<McpToolSnapshot[]>;
    probe(server: AiMcpServer): Promise<McpToolSnapshot[]>;
    reconnect(id: string): Promise<RespAiMcpServerRuntime>;
    installMarket(req: ReqMcpMarketInstall): Promise<AiMcpServer>;
}
