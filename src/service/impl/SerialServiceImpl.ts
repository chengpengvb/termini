import {inject, injectable} from "inversify";
import {Dispatch, SetStateAction} from "react";
import {API_BASE_URL} from "@/runtime-config.ts";
import {SerialService} from "@/service/SerialService.ts";
import type {ConfigSerial} from "@/common/model/ConfigSerial.ts";
import {AuthServiceImpl} from "@/service/impl/AuthServiceImpl.ts";
import {StepEvent} from "@/page/common/dto/terminal/StepEvent.ts";
import {REQ_WS_EVENT_SERIAL} from "@/common/model/req/ReqWsEvent.ts";
import {DeviceServiceImpl} from "@/service/impl/DeviceServiceImpl.ts";
import {RespSocketContext} from "@/common/model/resp/RespWsContext.ts";
import {BaseApiServiceImpl} from "@/service/impl/BaseApiServiceImpl.ts";
import {WebSocketServiceImpl} from "@/service/impl/WebSocketServiceImpl.ts";
import {BaseTerminalServiceImpl} from "@/service/impl/TerminalServiceImpl.ts";
import {RESP_EVENT_SERIAL} from "@/common/model/resp/event/RespEventSerial.ts";
import {BizError, ResultModel, SUCCESS_CODE} from "@/common/model/resp/ResultModel.ts";

@injectable()
export class SerialServiceImpl extends BaseTerminalServiceImpl implements SerialService {
    protected onOpenStepType: number = -1;
    protected onDeviceOsTypeType: number = -1;
    protected setWindowSizeType: number = -1;
    protected openType: number = REQ_WS_EVENT_SERIAL.OPEN;
    protected closeType: number = REQ_WS_EVENT_SERIAL.CLOSE;
    protected writeType: number = REQ_WS_EVENT_SERIAL.WRITE;

    protected onOpenType: number = RESP_EVENT_SERIAL.ON_OPEN;
    protected onCloseType: number = RESP_EVENT_SERIAL.ON_CLOSE;
    protected onErrorType: number = RESP_EVENT_SERIAL.ON_ERROR;
    protected onDataType: number = RESP_EVENT_SERIAL.ON_DATA;

    constructor(
        @inject(AuthServiceImpl) authService: AuthServiceImpl,
        @inject(DeviceServiceImpl) deviceService: DeviceServiceImpl,
        @inject(WebSocketServiceImpl) webSocketService: WebSocketServiceImpl,) {
        super(authService, webSocketService, deviceService);
    }

    protected parser2progress(_context: RespSocketContext): StepEvent {
        throw new Error("Method not implemented.");
    }

    async handleConnect(session_id: string, payload: any, handle: any, setOpened: Dispatch<SetStateAction<boolean>>): Promise<() => void> {
        const handleData = handle.onData((data: string) => {
            this.writeData(session_id, {data});
        });
        const handleResp = this.addWsListener(session_id, (resp: RespSocketContext) => {
            this.handleRespContext(handle, resp, setOpened);
        });
        this.handleOpen(session_id, payload).then();
        return () => {
            handleData();
            handleResp();
            this.close(session_id);
        };
    }

    async ports(): Promise<string[]> {
        const res = await fetch(`${API_BASE_URL}/serial/ports`, {headers: BaseApiServiceImpl.buildAuthHeaders()});
        if (!res.ok) {
            throw new Error(`HTTP ${res.status}`);
        }
        const body = (await res.json()) as ResultModel<string[]>;
        if (body.code !== SUCCESS_CODE) {
            throw new BizError(body.code);
        }
        if (body.data == null) {
            console.warn('[serial] ports data missing');
            return [];
        }
        return body.data;
    }

    async config(portName: string): Promise<ConfigSerial | null> {
        const res = await fetch(`${API_BASE_URL}/serial/config/get`, {
            method: 'POST',
            body: JSON.stringify({port_name: portName}),
            headers: {'Content-Type': 'application/json', ...BaseApiServiceImpl.buildAuthHeaders()},
        });
        if (!res.ok) {
            throw new Error(`HTTP ${res.status}`);
        }
        const body = (await res.json()) as ResultModel<ConfigSerial | null>;
        if (body.code !== SUCCESS_CODE) {
            throw new BizError(body.code);
        }
        return body.data ?? null;
    }

    async saveConfig(config: ConfigSerial): Promise<ConfigSerial> {
        const res = await fetch(`${API_BASE_URL}/serial/config/save`, {
            method: 'POST',
            body: JSON.stringify(config),
            headers: {'Content-Type': 'application/json', ...BaseApiServiceImpl.buildAuthHeaders()},
        });
        if (!res.ok) {
            throw new Error(`HTTP ${res.status}`);
        }
        const body = (await res.json()) as ResultModel<ConfigSerial>;
        if (body.code !== SUCCESS_CODE) {
            throw new BizError(body.code);
        }
        if (body.data == null) {
            throw new Error('serial config data missing');
        }
        return body.data;
    }

    addPortsChangeListener(cb: (ports: string[]) => void): () => void {
        return this.webSocketService.addWsListener((context: RespSocketContext) => {
            if (context.type !== RESP_EVENT_SERIAL.ON_PORTS_CHANGED) {
                return;
            }
            cb(context.payload as string[]);
        });
    }
}
