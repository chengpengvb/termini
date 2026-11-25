import {TerminalService} from "@/service/TerminalService.ts";
import {SshAuthAnswerEvent} from "@/common/model/req/event/WsSsh.ts";

export interface RdpService extends TerminalService {
    authAnswer(session_id: string, event: SshAuthAnswerEvent): void;
}
