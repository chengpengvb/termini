import {inject, injectable} from "inversify";
import {translate} from "@/common/i18n/i18n.ts";
import {TelnetService} from "@/service/TelnetService.ts";
import {AuthServiceImpl} from "@/service/impl/AuthServiceImpl.ts";
import {StepEvent} from "@/page/common/dto/terminal/StepEvent.ts";
import {RespSocketContext} from "@/common/model/resp/RespWsContext";
import {REQ_WS_EVENT_TELNET} from "@/common/model/req/ReqWsEvent.ts";
import {DeviceServiceImpl} from "@/service/impl/DeviceServiceImpl.ts";
import {OpenStep} from "@/common/model/resp/event/RespEventOpenStep.ts";
import {WebSocketServiceImpl} from "@/service/impl/WebSocketServiceImpl.ts";
import {BaseTerminalServiceImpl} from "@/service/impl/TerminalServiceImpl.ts";
import {RESP_EVENT_TELNET, RESP_TELNET_OPEN_STEP_TYPE} from "@/common/model/resp/event/RespEventTelnet.ts";

@injectable()
export class TelnetServiceImpl extends BaseTerminalServiceImpl implements TelnetService {
    protected openType: number = REQ_WS_EVENT_TELNET.OPEN;
    protected closeType: number = REQ_WS_EVENT_TELNET.CLOSE;
    protected writeType: number = REQ_WS_EVENT_TELNET.WRITE;
    protected setWindowSizeType: number = REQ_WS_EVENT_TELNET.RESIZE;

    protected onOpenType: number = RESP_EVENT_TELNET.ON_OPEN;
    protected onCloseType: number = RESP_EVENT_TELNET.ON_CLOSE;
    protected onErrorType: number = RESP_EVENT_TELNET.ON_ERROR;
    protected onDataType: number = RESP_EVENT_TELNET.ON_DATA;
    protected onOpenStepType: number = RESP_EVENT_TELNET.ON_OPEN_STEP;
    protected onDeviceOsTypeType: number = RESP_EVENT_TELNET.ON_DEVICE_OS_TYPE;

    private readonly stepTypeError: Set<number> = new Set([
        RESP_TELNET_OPEN_STEP_TYPE.ERROR_TCP_FAILED,
        RESP_TELNET_OPEN_STEP_TYPE.ERROR_PROBE_FAILED,
    ]);

    private readonly stepTypeMessageMap: Record<number, string> = {
        [RESP_TELNET_OPEN_STEP_TYPE.PROBE_START]: 'protocol.openStep.telnet.probeStart',
        [RESP_TELNET_OPEN_STEP_TYPE.PROBE_TCP_CONNECTING]: 'protocol.openStep.telnet.probeTcpConnecting',
        [RESP_TELNET_OPEN_STEP_TYPE.PROBE_TCP_CONNECTED]: 'protocol.openStep.telnet.probeTcpConnected',
        [RESP_TELNET_OPEN_STEP_TYPE.PROBE_NEGOTIATING]: 'protocol.openStep.telnet.probeNegotiating',
        [RESP_TELNET_OPEN_STEP_TYPE.PROBE_COMPLETED]: 'protocol.openStep.telnet.probeCompleted',
        [RESP_TELNET_OPEN_STEP_TYPE.SESSION_OPENED]: 'protocol.openStep.telnet.sessionOpened',
        [RESP_TELNET_OPEN_STEP_TYPE.ERROR_TCP_FAILED]: 'protocol.openStep.telnet.errorTcpFailed',
        [RESP_TELNET_OPEN_STEP_TYPE.ERROR_PROBE_FAILED]: 'protocol.openStep.telnet.errorProbeFailed',
    }

    constructor(
        @inject(AuthServiceImpl) authService: AuthServiceImpl,
        @inject(DeviceServiceImpl) deviceService: DeviceServiceImpl,
        @inject(WebSocketServiceImpl) webSocketService: WebSocketServiceImpl,) {
        super(authService, webSocketService, deviceService);
    }

    private type2StepTypeMessage(step: OpenStep): string {
        const stepType = step.step_type;
        const key = this.stepTypeMessageMap[stepType];
        if (!key) {
            return String()
        }
        return translate(key);
    }

    protected parser2progress(context: RespSocketContext): StepEvent {
        const step = context.payload as OpenStep;
        const stepTypeMessage = this.type2StepTypeMessage(step);
        const isError = this.stepTypeError.has(step.step_type);
        const opened = step.step_type === RESP_TELNET_OPEN_STEP_TYPE.SESSION_OPENED;
        return {
            stop: isError,
            opened: opened,
            error: isError,
            data: step.data,
            progress: step.progress,
            stepTypeMessage: stepTypeMessage,
        };
    }

}