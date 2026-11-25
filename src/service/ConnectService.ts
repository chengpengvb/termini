import {Dispatch, SetStateAction} from "react";
import {BaseService} from "@/service/BaseService.ts";
import {StepEvent} from "@/page/common/dto/terminal/StepEvent.ts";
import {RespSocketContext} from "@/common/model/resp/RespWsContext.ts";

export interface ConnectService extends BaseService {

    noticeRetry(session_id: string): void;

    addCloseListener(session_id: string, cb: () => void): () => void;

    addConnectRetryListener(session_id: string, cb: () => void): () => void;

    addAuthListener(session_id: string, cb: (context: RespSocketContext) => void): () => void;

    addConnectStepListener(session_id: string, cb: (stepEvent: StepEvent) => void): () => void;

    handleConnect(session_id: string, payload: any, handle: any, setOpened: Dispatch<SetStateAction<boolean>>): Promise<() => void>;

}

