import {inject, injectable} from "inversify";
import {API_BASE_URL} from "@/runtime-config.ts";
import {ShellDescr} from "@/common/model/ShellDescr";
import {ShellService} from "@/service/ShellService.ts";
import {AuthServiceImpl} from "@/service/impl/AuthServiceImpl.ts";
import {StepEvent} from "@/page/common/dto/terminal/StepEvent.ts";
import {REQ_WS_EVENT_SHELL} from "@/common/model/req/ReqWsEvent.ts";
import {DeviceServiceImpl} from "@/service/impl/DeviceServiceImpl.ts";
import {RespSocketContext} from "@/common/model/resp/RespWsContext.ts";
import {BaseApiServiceImpl} from "@/service/impl/BaseApiServiceImpl.ts";
import {WebSocketServiceImpl} from "@/service/impl/WebSocketServiceImpl.ts";
import {RESP_EVENT_SHELL} from "@/common/model/resp/event/RespEventShell.ts";
import {BaseTerminalServiceImpl} from "@/service/impl/TerminalServiceImpl.ts";
import {BizError, ResultModel, SUCCESS_CODE} from "@/common/model/resp/ResultModel.ts";

@injectable()
export class ShellServiceImpl extends BaseTerminalServiceImpl implements ShellService {
    protected onOpenStepType: number = -1;
    protected onDeviceOsTypeType: number = -1;
    protected openType: number = REQ_WS_EVENT_SHELL.OPEN;
    protected closeType: number = REQ_WS_EVENT_SHELL.CLOSE;
    protected writeType: number = REQ_WS_EVENT_SHELL.WRITE;
    protected setWindowSizeType: number = REQ_WS_EVENT_SHELL.RESIZE;
    protected override readonly snapshotType: number = REQ_WS_EVENT_SHELL.SNAPSHOT;

    protected onOpenType: number = RESP_EVENT_SHELL.ON_OPEN;
    protected onCloseType: number = RESP_EVENT_SHELL.ON_CLOSE;
    protected onErrorType: number = RESP_EVENT_SHELL.ON_ERROR;
    protected onDataType: number = RESP_EVENT_SHELL.ON_DATA;
    protected override readonly onExitType: number = RESP_EVENT_SHELL.EXIT;
    protected override readonly onSnapshotRequestType: number = RESP_EVENT_SHELL.SNAPSHOT_REQUEST;

    constructor(
        @inject(AuthServiceImpl) authService: AuthServiceImpl,
        @inject(DeviceServiceImpl) deviceService: DeviceServiceImpl,
        @inject(WebSocketServiceImpl) webSocketService: WebSocketServiceImpl,) {
        super(authService, webSocketService, deviceService);
    }

    protected parser2progress(_context: RespSocketContext): StepEvent {
        throw new Error("Method not implemented.");
    }

    async discover(): Promise<ShellDescr[]> {
        const res = await fetch(`${API_BASE_URL}/shell/discover`, {headers: BaseApiServiceImpl.buildAuthHeaders()});
        if (!res.ok) {
            throw new Error(`HTTP ${res.status}`);
        }
        const body = (await res.json()) as ResultModel<ShellDescr[]>;
        if (body.code !== SUCCESS_CODE) {
            throw new BizError(body.code);
        }
        return body.data ?? [];
    }

}