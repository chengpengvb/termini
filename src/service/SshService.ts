import {TerminalService} from "@/service/TerminalService.ts";
import {StepEvent} from "@/page/common/dto/terminal/StepEvent.ts";
import {SshAuthAnswerEvent} from "@/common/model/req/event/WsSsh.ts";
import {RespSocketContext} from "@/common/model/resp/RespWsContext.ts";

export interface SshService extends TerminalService {
    isAuthType(type: number): boolean;
    parser2progress(context: RespSocketContext): StepEvent;
    authAnswer(session_id: string, event: SshAuthAnswerEvent): void;
}
