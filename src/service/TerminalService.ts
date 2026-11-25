import {ConnectService} from "@/service/ConnectService.ts";
import {RespSocketContext} from "@/common/model/resp/RespWsContext.ts";

export interface TerminalService extends ConnectService {

    handleClose(session_id: string): void;

    writeData(session_id: string, payload: any): void;

    handleOpen(session_id: string, payload: any): void;

    setWindowSize(session_id: string, payload: any): void;

    addWsListener(session_id: string, cb: (context: RespSocketContext) => void): () => void;

}

