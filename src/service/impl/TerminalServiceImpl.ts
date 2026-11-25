import {injectable} from "inversify";
import {Dispatch, SetStateAction} from "react";
import {WindowSize} from "@/common/model/dto/WindowSize.ts";
import {TerminalService} from "@/service/TerminalService.ts";
import {StepEvent} from "@/page/common/dto/terminal/StepEvent";
import {ReqSocketContext} from "@/common/model/req/ReqWsContext";
import {AuthServiceImpl} from "@/service/impl/AuthServiceImpl.ts";
import {BaseServiceImpl} from "@/service/impl/BaseServiceImpl.ts";
import {RespSocketContext} from "@/common/model/resp/RespWsContext";
import {DeviceServiceImpl} from "@/service/impl/DeviceServiceImpl.ts";
import {TerminalHandle} from "@/page/common/dto/terminal/TerminalHandle.ts";
import {WebSocketServiceImpl} from "@/service/impl/WebSocketServiceImpl.ts";

@injectable()
export abstract class BaseTerminalServiceImpl extends BaseServiceImpl implements TerminalService {

    protected readonly onExitType: number = -1;
    protected abstract readonly onDataType: number;
    protected abstract readonly onOpenType: number;
    protected abstract readonly onCloseType: number;
    protected abstract readonly onErrorType: number;
    protected abstract readonly onOpenStepType: number;
    protected abstract readonly onDeviceOsTypeType: number;
    protected readonly onSnapshotRequestType: number = -1;

    protected readonly snapshotType: number = -1;
    protected abstract readonly openType: number;
    protected abstract readonly closeType: number;
    protected abstract readonly writeType: number;
    protected abstract readonly setWindowSizeType: number;

    protected abstract parser2progress(context: RespSocketContext): StepEvent;

    private readonly closeListeners: Map<string, Set<() => void>> = new Map();
    private readonly connectRetryListeners: Map<string, Set<() => void>> = new Map();

    protected constructor(
        protected readonly authService: AuthServiceImpl,
        protected readonly webSocketService: WebSocketServiceImpl,
        protected readonly deviceService: DeviceServiceImpl,
    ) {
        super();
    }

    protected isAuthType(_type: number): boolean {
        return false
    }

    addAuthListener(session_id: string, cb: (context: RespSocketContext) => void): () => void {
        return this.authService.addAuthListener(session_id, cb);
    }

    addConnectStepListener(session_id: string, cb: (stepEvent: StepEvent) => void): () => void {
        return this.authService.addConnectStepListener(session_id, cb);
    }

    addCloseListener(session_id: string, cb: () => void): () => void {
        return this.authService.addSessionListener(this.closeListeners, session_id, cb);
    }

    addConnectRetryListener(session_id: string, cb: () => void): () => void {
        return this.authService.addSessionListener(this.connectRetryListeners, session_id, cb);
    }

    send(message: ReqSocketContext): void {
        this.webSocketService.send(message);
    }

    async handleOpen(session_id: string, payload: any) {
        this.authService.clearConnectSteps(session_id);
        const message: ReqSocketContext = {
            type: this.openType,
            payload: payload,
            session_id: session_id,
        };
        this.webSocketService.send(message);
    }

    writeData(session_id: string, payload: any): void {
        const message: ReqSocketContext = {
            payload: payload,
            type: this.writeType,
            session_id: session_id,
        };
        this.webSocketService.send(message);
    }

    setWindowSize(session_id: string, payload: any): void {
        const message: ReqSocketContext = {
            payload: payload,
            session_id: session_id,
            type: this.setWindowSizeType,
        };
        this.webSocketService.send(message);
    }

    handleClose(session_id: string): void {
        const message: ReqSocketContext = {
            type: this.closeType,
            session_id: session_id,
        };
        this.webSocketService.send(message);
    }

    addWsListener(session_id: string, cb: (context: RespSocketContext) => void): () => void {
        return this.webSocketService.addWsListener((context) => {
            if (context.session_id !== session_id) {
                return;
            }
            cb(context);
        });
    }

    async handleConnect(session_id: string, payload: any, handle: any, setOpened: Dispatch<SetStateAction<boolean>>): Promise<() => void> {
        const handleData = handle.onData((data: string) => {
            this.handleOnData(session_id, data)
        });
        const handleResize = handle.onWindowResize((nextSize: WindowSize) => {
            this.handleWindowResize(session_id, nextSize)
        });
        const handleResp = this.addWsListener(session_id, (resp: RespSocketContext) => {
            this.handleRespContext(handle, resp, setOpened)
        });
        this.handleOpen(session_id, payload).then()
        return () => {
            handleData()
            handleResp()
            handleResize()
            this.close(session_id)
        }
    }

    close(session_id: string) {
        this.handleClose(session_id)
    }

    private handleOnData(session_id: string, data: string) {
        const payload = {
            data: data,
        }
        this.writeData(session_id, payload)
    }

    handleWindowResize(session_id: string, size: WindowSize) {
        const payload = {
            data: size,
        }
        this.setWindowSize(session_id, payload)
    }

    private reportSnapshot(session_id: string, terminal: TerminalHandle): void {
        if (this.snapshotType < 0) {
            return
        }
        const screen = terminal.getScreen()
        const message: ReqSocketContext = {
            type: this.snapshotType,
            session_id: session_id,
            payload: {data: {text: screen.text, cursorRow: screen.cursorRow, cursorCol: screen.cursorCol}},
        };
        this.webSocketService.send(message);
    }

    noticeRetry(session_id: string): void {
        this.authService.emitEvent(this.connectRetryListeners, session_id, undefined)
    }

    handleRespContext(terminal: TerminalHandle, context: RespSocketContext, setOpened: Dispatch<SetStateAction<boolean>>) {
        const type = context.type
        if (type == this.onDataType) {
            terminal.write(new Uint8Array(context.payload as number[]))
        } else if (type == this.onSnapshotRequestType) {
            this.reportSnapshot(context.session_id, terminal)
        } else if (type == this.onOpenType) {
            setOpened(true)
        } else if (type == this.onCloseType || type == this.onExitType) {
            setOpened(false)
            this.authService.emitEvent(this.closeListeners, context.session_id, undefined)
        } else if (type == this.onOpenStepType) {
            this.authService.onOpenStep(context, this.isAuthType.bind(this))
            this.authService.notifyConnectStep(context.session_id, this.parser2progress(context))
        } else if (type == this.onErrorType) {
            //TODO 错误标识
        } else if (type == this.onDeviceOsTypeType) {
            this.deviceService.selectAllWithConfig().then()
        }
    }


}