import {inject, injectable} from "inversify";
import {translate} from "@/common/i18n/i18n.ts";
import {SshService} from "@/service/SshService.ts";
import {AuthServiceImpl} from "@/service/impl/AuthServiceImpl.ts";
import {StepEvent} from "@/page/common/dto/terminal/StepEvent.ts";
import {ReqSocketContext} from "@/common/model/req/ReqWsContext.ts";
import {RespSocketContext} from "@/common/model/resp/RespWsContext";
import {SshAuthAnswerEvent} from "@/common/model/req/event/WsSsh.ts";
import {DeviceServiceImpl} from "@/service/impl/DeviceServiceImpl.ts";
import {OpenStep} from "@/common/model/resp/event/RespEventOpenStep.ts";
import {RESP_EVENT_SSH} from "@/common/model/resp/event/RespEventSsh.ts";
import {WebSocketServiceImpl} from "@/service/impl/WebSocketServiceImpl.ts";
import {BaseTerminalServiceImpl} from "@/service/impl/TerminalServiceImpl.ts";
import {RESP_SSH_OPEN_STEP_TYPE} from "@/common/model/resp/event/RespEventSshAuth.ts";
import {AUTO_TYPE_MAP, STEP_TYPE_MESSAGE_MAP} from "@/common/model/dto/SshAuthType.ts";
import {REQ_WS_EVENT_SSH, REQ_WS_EVENT_SSH_AUTH} from "@/common/model/req/ReqWsEvent.ts";

@injectable()
export class SshServiceImpl extends BaseTerminalServiceImpl implements SshService {

    protected openType: number = REQ_WS_EVENT_SSH.OPEN;
    protected closeType: number = REQ_WS_EVENT_SSH.CLOSE;
    protected writeType: number = REQ_WS_EVENT_SSH.WRITE;
    protected setWindowSizeType: number = REQ_WS_EVENT_SSH.RESIZE;

    protected onOpenType: number = RESP_EVENT_SSH.ON_OPEN;
    protected onCloseType: number = RESP_EVENT_SSH.ON_CLOSE;
    protected onErrorType: number = RESP_EVENT_SSH.ON_ERROR;
    protected onDataType: number = RESP_EVENT_SSH.ON_DATA;
    protected onOpenStepType: number = RESP_EVENT_SSH.ON_OPEN_STEP;
    protected onDeviceOsTypeType: number = RESP_EVENT_SSH.ON_DEVICE_OS_TYPE;

    private readonly stepTypeError: Set<number> = new Set([
        RESP_SSH_OPEN_STEP_TYPE.ERROR_TCP_FAILED,
        RESP_SSH_OPEN_STEP_TYPE.ERROR_PROBE_FAILED,
        RESP_SSH_OPEN_STEP_TYPE.ERROR_AUTH_FAILED,
        RESP_SSH_OPEN_STEP_TYPE.ERROR_AUTH_TIMEOUT,
        RESP_SSH_OPEN_STEP_TYPE.ERROR_CHANNEL_FAILED,
        RESP_SSH_OPEN_STEP_TYPE.ERROR_OPEN_CANCELLED,
    ]);

    constructor(
        @inject(AuthServiceImpl) authService: AuthServiceImpl,
        @inject(DeviceServiceImpl) deviceService: DeviceServiceImpl,
        @inject(WebSocketServiceImpl) webSocketService: WebSocketServiceImpl,) {
        super(authService, webSocketService, deviceService);
    }

    authAnswer(session_id: string, event: SshAuthAnswerEvent): void {
        const message: ReqSocketContext = {
            payload: event,
            session_id: session_id,
            type: REQ_WS_EVENT_SSH_AUTH.AUTH_ANSWER,
        };
        this.webSocketService.send(message);
    }

    isAuthType(type: number): boolean {
        return AUTO_TYPE_MAP.has(type);
    }

    private type2StepTypeMessage(step: OpenStep): string {
        const stepType = step.step_type;
        const key = STEP_TYPE_MESSAGE_MAP[stepType];
        if (!key) {
            return String()
        }
        return translate(key);
    }

    parser2progress(context: RespSocketContext): StepEvent {
        let data = null
        const step = context.payload as OpenStep;
        const stepTypeMessage = this.type2StepTypeMessage(step);
        const isError = this.stepTypeError.has(step.step_type);
        const opened = step.step_type === RESP_SSH_OPEN_STEP_TYPE.SESSION_OPENED;
        if (isError) {
            data = step.data
        }
        return {
            data: data,
            stop: isError,
            opened: opened,
            error: isError,
            progress: step.progress,
            stepTypeMessage: stepTypeMessage,
        };
    }

}