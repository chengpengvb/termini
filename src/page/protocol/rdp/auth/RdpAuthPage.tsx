import {PROTOCOL} from "@/common/const/Const.ts";
import {ConnectService} from "@/service/ConnectService.ts";
import {useRdpService} from "@/common/context/useAppHooks.tsx";
import type {ReqDevice} from "@/common/model/req/ReqDevice.ts";
import {OpenStep} from "@/common/model/resp/event/RespEventOpenStep.ts";
import {KnownHost} from "@/page/protocol/common/auth/pages/KnownHost.tsx";
import {PasswordAuth} from "@/page/protocol/common/auth/pages/PasswordAuth.tsx";
import {UsernameAuth} from "@/page/protocol/common/auth/pages/UsernameAuth.tsx";
import {RESP_RDP_OPEN_STEP_TYPE} from "@/common/model/resp/event/RespEventRdp.ts";
import {AuthDialog, type AuthStepContext} from "@/page/protocol/common/auth/AuthDialog.tsx";

export interface RdpAuthProps {
    sessionId: string;
    reqDevice: ReqDevice;
    connectService?: ConnectService;
}

export function RdpAuthPage(props: RdpAuthProps) {
    const {sessionId, reqDevice, connectService} = props;
    const rdpService = useRdpService();

    const renderStep = (context: AuthStepContext) => {
        const {respSocketContext, handleClose} = context;
        const authPageProps = {
            reqDevice: context.reqDevice,
            respSocketContext,
            handleClose,
            setAlertInfo: context.setAlertInfo,
            protocol: PROTOCOL.RDP,
        };
        switch ((respSocketContext.payload as OpenStep).step_type) {
            case RESP_RDP_OPEN_STEP_TYPE.AUTH_USERNAME_INPUT:
                return <UsernameAuth {...authPageProps} isRetry={false}/>;
            case RESP_RDP_OPEN_STEP_TYPE.AUTH_USERNAME_RETRY:
                return <UsernameAuth {...authPageProps} isRetry={true}/>;
            case RESP_RDP_OPEN_STEP_TYPE.AUTH_PASSWORD_INPUT:
                return <PasswordAuth {...authPageProps} isRetry={false}/>;
            case RESP_RDP_OPEN_STEP_TYPE.AUTH_PASSWORD_RETRY:
                return <PasswordAuth {...authPageProps} isRetry={true}/>;
            case RESP_RDP_OPEN_STEP_TYPE.CERT_FINGERPRINT_CONFIRM:
                return <KnownHost respSocketContext={respSocketContext} handleClose={handleClose} onAnswer={(session_id, event) => rdpService.authAnswer(session_id, event)}/>;
            default:
                return null;
        }
    };

    return <AuthDialog sessionId={sessionId} reqDevice={reqDevice} connectService={connectService} renderStep={renderStep}/>;
}
