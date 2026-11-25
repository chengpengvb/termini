import {BaseService} from "@/service/BaseService.ts";
import {StepEvent} from "@/page/common/dto/terminal/StepEvent.ts";
import {RespSocketContext} from "@/common/model/resp/RespWsContext.ts";

export interface AuthService extends BaseService {

    clearConnectSteps(session_id: string): void;

    notifyConnectStep(session_id: string, stepEvent: StepEvent): void;

    onOpenStep(context: RespSocketContext, isAuthType: (type: number) => boolean): void;

    addAuthListener(session_id: string, cb: (context: RespSocketContext) => void): () => void;

    addConnectStepListener(session_id: string, cb: (stepEvent: StepEvent) => void): () => void;

    emitEvent<TArg>(store: Map<string, Set<(arg: TArg) => void>>, session_id: string, arg: TArg): void;

    addSessionListener<TCb extends (...args: any[]) => void>(store: Map<string, Set<TCb>>, session_id: string, cb: TCb): () => void;

}
