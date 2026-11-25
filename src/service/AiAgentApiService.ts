import type {BaseService} from './BaseService';
import type {AiSkill} from '@/common/model/AiSkill.ts';
import type {ReqShellDaemonStop} from '@/common/model/req/ReqShellDaemon.ts';
import type {RespAgentShellDaemonRow} from '@/common/model/resp/RespAgent.ts';
import type {ReqCancelGenerateSkill, ReqGenerateSkill} from '@/common/model/req/ReqGenerateSkill.ts';

export interface AiAgentApiService extends BaseService {

    stopShellDaemon(req: ReqShellDaemonStop): Promise<void>;

    cancelGenerateSkill(req: ReqCancelGenerateSkill): Promise<void>;

    listShellDaemons(chatId: string): Promise<RespAgentShellDaemonRow[]>;

    generateSkill(req: ReqGenerateSkill, signal?: AbortSignal): Promise<AiSkill>;

}
