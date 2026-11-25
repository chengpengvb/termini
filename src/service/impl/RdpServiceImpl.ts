import {inject, injectable} from "inversify";
import {Dispatch, SetStateAction} from "react";
import {translate} from "@/common/i18n/i18n.ts";
import {RdpService} from "@/service/RdpService.ts";
import {RdpFocus} from "@/common/model/req/event/WsRdp.ts";
import {WindowSize} from "@/common/model/dto/WindowSize.ts";
import {RdpHandle} from "@/page/common/dto/rdp/RdpHandle.ts";
import {rdpFrameTimestamp} from "@/page/common/util/rdpUtil.ts";
import {ReqSocketContext} from "@/common/model/req/ReqWsContext";
import {AuthServiceImpl} from "@/service/impl/AuthServiceImpl.ts";
import {REQ_WS_EVENT_RDP} from "@/common/model/req/ReqWsEvent.ts";
import {StepEvent} from "@/page/common/dto/terminal/StepEvent.ts";
import {RespSocketContext} from "@/common/model/resp/RespWsContext";
import {SshAuthAnswerEvent} from "@/common/model/req/event/WsSsh.ts";
import {DeviceServiceImpl} from "@/service/impl/DeviceServiceImpl.ts";
import {OpenStep} from "@/common/model/resp/event/RespEventOpenStep.ts";
import {WebSocketServiceImpl} from "@/service/impl/WebSocketServiceImpl.ts";
import {BaseTerminalServiceImpl} from "@/service/impl/TerminalServiceImpl.ts";
import {FingerprintPrompt, RetryPrompt} from "@/common/model/dto/SshAuthPrompt.ts";
import {RESP_EVENT_RDP, RESP_RDP_OPEN_STEP_TYPE, type RespRdpDesktopSize} from "@/common/model/resp/event/RespEventRdp.ts";

@injectable()
export class RdpServiceImpl extends BaseTerminalServiceImpl implements RdpService {
    protected openType: number = REQ_WS_EVENT_RDP.OPEN;
    protected closeType: number = REQ_WS_EVENT_RDP.CLOSE;
    protected writeType: number = REQ_WS_EVENT_RDP.INPUT;
    protected setWindowSizeType: number = REQ_WS_EVENT_RDP.RESIZE;

    protected onDataType: number = -1;
    protected onDeviceOsTypeType: number = -1;
    protected onOpenType: number = RESP_EVENT_RDP.ON_OPEN;
    protected onCloseType: number = RESP_EVENT_RDP.ON_CLOSE;
    protected onErrorType: number = RESP_EVENT_RDP.ON_ERROR;
    protected onOpenStepType: number = RESP_EVENT_RDP.ON_OPEN_STEP;

    private readonly stepTypeError: Set<number> = new Set([
        RESP_RDP_OPEN_STEP_TYPE.ERROR_TCP_FAILED,
        RESP_RDP_OPEN_STEP_TYPE.ERROR_NEGOTIATE_FAILED,
        RESP_RDP_OPEN_STEP_TYPE.ERROR_TLS_FAILED,
        RESP_RDP_OPEN_STEP_TYPE.ERROR_AUTH_FAILED,
        RESP_RDP_OPEN_STEP_TYPE.ERROR_MCS_FAILED,
        RESP_RDP_OPEN_STEP_TYPE.ERROR_LICENSE_FAILED,
        RESP_RDP_OPEN_STEP_TYPE.ERROR_CAPABILITIES_FAILED,
        RESP_RDP_OPEN_STEP_TYPE.ERROR_ACTIVATE_FAILED,
    ]);

    private readonly stepTypeMessageMap: Record<number, string> = {
        [RESP_RDP_OPEN_STEP_TYPE.TCP_CONNECTING]: 'protocol.openStep.rdp.tcpConnecting',
        [RESP_RDP_OPEN_STEP_TYPE.TCP_CONNECTED]: 'protocol.openStep.rdp.tcpConnected',
        [RESP_RDP_OPEN_STEP_TYPE.NEGOTIATING]: 'protocol.openStep.rdp.negotiating',
        [RESP_RDP_OPEN_STEP_TYPE.TLS_HANDSHAKING]: 'protocol.openStep.rdp.tlsHandshaking',
        [RESP_RDP_OPEN_STEP_TYPE.TLS_ESTABLISHED]: 'protocol.openStep.rdp.tlsEstablished',
        [RESP_RDP_OPEN_STEP_TYPE.AUTHENTICATING]: 'protocol.openStep.rdp.authenticating',
        [RESP_RDP_OPEN_STEP_TYPE.AUTHENTICATED]: 'protocol.openStep.rdp.authenticated',
        [RESP_RDP_OPEN_STEP_TYPE.MCS_CONNECTING]: 'protocol.openStep.rdp.mcsConnecting',
        [RESP_RDP_OPEN_STEP_TYPE.MCS_CONNECTED]: 'protocol.openStep.rdp.mcsConnected',
        [RESP_RDP_OPEN_STEP_TYPE.LICENSING]: 'protocol.openStep.rdp.licensing',
        [RESP_RDP_OPEN_STEP_TYPE.CAPABILITIES_EXCHANGING]: 'protocol.openStep.rdp.capabilitiesExchanging',
        [RESP_RDP_OPEN_STEP_TYPE.ACTIVATING]: 'protocol.openStep.rdp.activating',
        [RESP_RDP_OPEN_STEP_TYPE.AUTH_USERNAME_INPUT]: 'protocol.openStep.rdp.authUsernameInput',
        [RESP_RDP_OPEN_STEP_TYPE.AUTH_USERNAME_RETRY]: 'protocol.openStep.rdp.authUsernameRetry',
        [RESP_RDP_OPEN_STEP_TYPE.AUTH_PASSWORD_INPUT]: 'protocol.openStep.rdp.authPasswordInput',
        [RESP_RDP_OPEN_STEP_TYPE.AUTH_PASSWORD_RETRY]: 'protocol.openStep.rdp.authPasswordRetry',
        [RESP_RDP_OPEN_STEP_TYPE.CERT_FINGERPRINT_CONFIRM]: 'protocol.openStep.rdp.certFingerprintConfirm',
        [RESP_RDP_OPEN_STEP_TYPE.SESSION_OPENED]: 'protocol.openStep.rdp.sessionOpened',
        [RESP_RDP_OPEN_STEP_TYPE.ERROR_TCP_FAILED]: 'protocol.openStep.rdp.errorTcpFailed',
        [RESP_RDP_OPEN_STEP_TYPE.ERROR_NEGOTIATE_FAILED]: 'protocol.openStep.rdp.errorNegotiateFailed',
        [RESP_RDP_OPEN_STEP_TYPE.ERROR_TLS_FAILED]: 'protocol.openStep.rdp.errorTlsFailed',
        [RESP_RDP_OPEN_STEP_TYPE.ERROR_AUTH_FAILED]: 'protocol.openStep.rdp.errorAuthFailed',
        [RESP_RDP_OPEN_STEP_TYPE.ERROR_MCS_FAILED]: 'protocol.openStep.rdp.errorMcsFailed',
        [RESP_RDP_OPEN_STEP_TYPE.ERROR_LICENSE_FAILED]: 'protocol.openStep.rdp.errorLicenseFailed',
        [RESP_RDP_OPEN_STEP_TYPE.ERROR_CAPABILITIES_FAILED]: 'protocol.openStep.rdp.errorCapabilitiesFailed',
        [RESP_RDP_OPEN_STEP_TYPE.ERROR_ACTIVATE_FAILED]: 'protocol.openStep.rdp.errorActivateFailed',
    }
    private readonly openedSessions: Set<string> = new Set();

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
            type: REQ_WS_EVENT_RDP.AUTH_ANSWER,
        };
        this.webSocketService.send(message);
    }

    protected isAuthType(type: number): boolean {
        return type === RESP_RDP_OPEN_STEP_TYPE.CERT_FINGERPRINT_CONFIRM
            || type === RESP_RDP_OPEN_STEP_TYPE.AUTH_USERNAME_INPUT
            || type === RESP_RDP_OPEN_STEP_TYPE.AUTH_USERNAME_RETRY
            || type === RESP_RDP_OPEN_STEP_TYPE.AUTH_PASSWORD_INPUT
            || type === RESP_RDP_OPEN_STEP_TYPE.AUTH_PASSWORD_RETRY;
    }

    private type2StepTypeMessage(step: OpenStep): string {
        const stepType = step.step_type;
        const key = this.stepTypeMessageMap[stepType];
        if (!key) {
            return String()
        }
        return translate(key);
    }

    private parseStepData(step: OpenStep): string | null {
        const data = step.data;
        if (data == null) {
            return null;
        }
        switch (step.step_type) {
            case RESP_RDP_OPEN_STEP_TYPE.AUTH_USERNAME_INPUT:
                return translate('protocol.auth.usernameNotConfigured');
            case RESP_RDP_OPEN_STEP_TYPE.AUTH_USERNAME_RETRY:
                return (data as RetryPrompt).last_error;
            case RESP_RDP_OPEN_STEP_TYPE.AUTH_PASSWORD_INPUT:
                return translate('protocol.auth.passwordNotConfigured');
            case RESP_RDP_OPEN_STEP_TYPE.AUTH_PASSWORD_RETRY:
                return (data as RetryPrompt).last_error;
            case RESP_RDP_OPEN_STEP_TYPE.CERT_FINGERPRINT_CONFIRM: {
                const prompt = data as FingerprintPrompt;
                return `${prompt.host}:${prompt.port} ${prompt.algorithm} ${prompt.sha256}`;
            }
            default:
                return data as string;
        }
    }

    protected parser2progress(context: RespSocketContext): StepEvent {
        const step = context.payload as OpenStep;
        const stepTypeMessage = this.type2StepTypeMessage(step);
        const isError = this.stepTypeError.has(step.step_type);
        const opened = step.step_type === RESP_RDP_OPEN_STEP_TYPE.SESSION_OPENED;
        return {
            stop: isError,
            opened: opened,
            error: isError,
            data: this.parseStepData(step),
            progress: step.progress,
            stepTypeMessage: stepTypeMessage,
        };
    }

    async handleConnect(session_id: string, payload: any, handle: any, setOpened: Dispatch<SetStateAction<boolean>>): Promise<() => void> {
        const dispose = await super.handleConnect(session_id, payload, handle, setOpened);
        const handleVisibility = (handle as RdpHandle).onVisibilityChange((visible: boolean) => {
            const message: ReqSocketContext = {
                session_id: session_id,
                payload: {data: visible},
                type: REQ_WS_EVENT_RDP.VISIBILITY,
            };
            this.webSocketService.send(message);
        });
        const handleFocus = (handle as RdpHandle).onFocusChange((focus: RdpFocus | null) => {
            const message: ReqSocketContext = {
                session_id: session_id,
                payload: {data: focus},
                type: REQ_WS_EVENT_RDP.FOCUS,
            };
            this.webSocketService.send(message);
        });
        return () => {
            this.openedSessions.delete(session_id);
            handleFocus();
            handleVisibility();
            dispose();
        };
    }

    handleWindowResize(session_id: string, size: WindowSize): void {
        if (!this.openedSessions.has(session_id)) {
            return;
        }
        super.handleWindowResize(session_id, size);
    }

    handleRespContext(handle: any, context: RespSocketContext, setOpened: Dispatch<SetStateAction<boolean>>) {
        const rdp = handle as RdpHandle;
        const type = context.type
        if (type == RESP_EVENT_RDP.ON_FRAME) {
            const payload = context.payload as ArrayBuffer;
            const timestamp = rdpFrameTimestamp(payload);
            void (async () => {
                try {
                    await rdp.drawFrame(payload);
                } finally {
                    this.webSocketService.send({
                        session_id: context.session_id,
                        payload: {data: timestamp},
                        type: REQ_WS_EVENT_RDP.FRAME_SYNC,
                    });
                }
            })();
        } else if (type == RESP_EVENT_RDP.ON_CURSOR) {
            rdp.setCursor(context.payload as ArrayBuffer)
        } else if (type == RESP_EVENT_RDP.ON_DESKTOP_RESIZE) {
            rdp.resizeSurface(context.payload as RespRdpDesktopSize)
        } else if (type == RESP_EVENT_RDP.ON_NETWORK) {
            rdp.setNetwork(context.payload as number)
        } else {
            super.handleRespContext(handle, context, setOpened)
            if (type == this.onOpenType) {
                this.openedSessions.add(context.session_id);
                this.handleWindowResize(context.session_id, rdp.getWindowSize());
            } else if (type == this.onCloseType) {
                this.openedSessions.delete(context.session_id);
            }
        }
    }

}
