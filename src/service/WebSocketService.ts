import {BaseService} from "@/service/BaseService.ts";
import {ReqSocketContext} from "@/common/model/req/ReqWsContext.ts";
import {RespSocketContext} from "@/common/model/resp/RespWsContext.ts";

export interface WebSocketService extends BaseService {
    isConnected(): boolean;

    getSocketId(): string | undefined;

    send(message: ReqSocketContext): void;

    addWsListener(cb: (context: RespSocketContext) => void): () => void;

    addConnectListener(cb: (socketId: string | null) => void): () => void;

    addDisconnectListener(cb: (socketId: string | null) => void): () => void;
}
