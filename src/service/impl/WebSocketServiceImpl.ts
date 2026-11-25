import {injectable} from "inversify";
import {API_BASE_URL} from "@/runtime-config.ts";
import {io, type Socket} from 'socket.io-client';
import {ENTITY_TYPE} from '@/common/const/EntityTypeConst.ts';
import {WebSocketService} from "@/service/WebSocketService.ts";
import {ReqSocketContext} from "@/common/model/req/ReqWsContext.ts";
import {RespSocketContext} from "@/common/model/resp/RespWsContext.ts";
import {BaseListenerService} from "@/service/impl/BaseListenerService.ts";

@injectable()
export class WebSocketServiceImpl extends BaseListenerService implements WebSocketService {
    private readonly WS_EVENT_MSG = 'msg';
    private socket: Socket | null = null;
    private socketId: string | null = null;
    private static readonly KEY_MSG = 'ws-msg';
    private static readonly KEY_CONNECT = 'ws-connect';
    private static readonly KEY_DISCONNECT = 'ws-disconnect';

    protected getNotifyEntityType(): number {
        return ENTITY_TYPE.NONE;
    }

    getPhase(): number {
        return 100;
    }

    async start(): Promise<void> {
        if (this.socket) {
            return;
        }
        const socket = io(API_BASE_URL, {
            transports: ['websocket'],
            autoConnect: true,
            reconnection: true,
        });
        this.bindSocketEvents(socket);
        this.socket = socket;
    }

    async quit(): Promise<void> {
        if (this.socket) {
            this.socket.disconnect();
            this.socket = null;
        }
    }

    isConnected(): boolean {
        if (this.socket) {
            return this.socket.connected;
        }
        return false;
    }

    getSocketId(): string | undefined {
        return this.socket?.id;
    }

    send(message: ReqSocketContext): void {
        const socket = this.requireSocket();
        socket.emit(this.WS_EVENT_MSG, message);
    }

    addWsListener(cb: (context: RespSocketContext) => void): () => void {
        return this.addListener(WebSocketServiceImpl.KEY_MSG, cb);
    }

    addDisconnectListener(cb: (socketId: string | null) => void): () => void {
        return this.addListener(WebSocketServiceImpl.KEY_DISCONNECT, cb);
    }

    addConnectListener(cb: (socketId: string | null) => void): () => void {
        return this.addListener(WebSocketServiceImpl.KEY_CONNECT, cb);
    }

    private bindSocketEvents(socket: Socket): void {
        socket.on('connect', () => {
            if (socket.id) {
                this.socketId = socket.id;
            }
            console.info('[ws] connected', {id: socket.id, url: API_BASE_URL});
            this.notifyListeners(WebSocketServiceImpl.KEY_CONNECT, this.socketId);
        });
        socket.on('disconnect', reason => {
            this.dispatchDisconnect(reason);
        });
        socket.on('connect_error', err => {
            console.error('[ws] connect_error', err);
        });
        socket.on(this.WS_EVENT_MSG, (env: RespSocketContext) => {
            this.dispatchIncoming(env);
        });
    }

    private dispatchIncoming(env: RespSocketContext): void {
        if (!env) {
            return;
        }
        const context: RespSocketContext = {
            ...env,
            socketId: this.getSocketId(),
        };
        this.notifyListeners(WebSocketServiceImpl.KEY_MSG, context);
    }

    private dispatchDisconnect(reason: string): void {
        console.warn('[ws] disconnected', {reason, socketId: this.socketId});
        this.notifyListeners(WebSocketServiceImpl.KEY_DISCONNECT, this.socketId);
    }

    private requireSocket(): Socket {
        if (!this.socket) {
            throw new Error('WebSocketService not started');
        }
        return this.socket;
    }
}
