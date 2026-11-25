import {ConnectService} from "@/service/ConnectService.ts";
import {useSshService} from "@/common/context/useAppHooks.tsx";
import type {ReqDevice} from "@/common/model/req/ReqDevice.ts";
import {OpenStep} from "@/common/model/resp/event/RespEventOpenStep.ts";
import {KnownHost} from "@/page/protocol/common/auth/pages/KnownHost.tsx";
import {KeyPairAuth} from "@/page/protocol/ssh/auth/pages/KeyPairAuth.tsx";
import {MethodSelect} from "@/page/protocol/ssh/auth/pages/MethodSelect.tsx";
import {PasswordAuth} from "@/page/protocol/common/auth/pages/PasswordAuth.tsx";
import {UsernameAuth} from "@/page/protocol/common/auth/pages/UsernameAuth.tsx";
import {PassphraseAuth} from "@/page/protocol/ssh/auth/pages/PassphraseAuth.tsx";
import {RESP_SSH_OPEN_STEP_TYPE} from "@/common/model/resp/event/RespEventSshAuth.ts";
import {SshAgentKeySelect} from "@/page/protocol/ssh/auth/pages/SshAgentKeySelect.tsx";
import {AuthDialog, type AuthStepContext} from "@/page/protocol/common/auth/AuthDialog.tsx";
import {KeyboardInteractiveAuth} from "@/page/protocol/ssh/auth/pages/KeyboardInteractiveAuth.tsx";

export interface AuthProps {
    sessionId: string;
    reqDevice: ReqDevice;
    connectService?: ConnectService;
}

export function AuthPage(props: AuthProps) {
    const {sessionId, reqDevice, connectService} = props;
    const sshService = useSshService();

    const renderStep = (context: AuthStepContext) => {
        const {respSocketContext, handleClose} = context;
        const authPageProps = {
            reqDevice: context.reqDevice,
            respSocketContext,
            handleClose,
            setAlertInfo: context.setAlertInfo,
        };
        switch ((respSocketContext.payload as OpenStep).step_type) {
            case RESP_SSH_OPEN_STEP_TYPE.AUTH_USERNAME_INPUT:
                return <UsernameAuth {...authPageProps} isRetry={false}/>;
            case RESP_SSH_OPEN_STEP_TYPE.AUTH_USERNAME_RETRY:
                return <UsernameAuth {...authPageProps} isRetry={true}/>;
            case RESP_SSH_OPEN_STEP_TYPE.AUTH_PASSWORD_INPUT:
                return <PasswordAuth {...authPageProps} isRetry={false}/>;
            case RESP_SSH_OPEN_STEP_TYPE.AUTH_PASSWORD_RETRY:
                return <PasswordAuth {...authPageProps} isRetry={true}/>;
            case RESP_SSH_OPEN_STEP_TYPE.AUTH_PASSPHRASE_INPUT:
                return <PassphraseAuth {...authPageProps} isRetry={false}/>;
            case RESP_SSH_OPEN_STEP_TYPE.AUTH_PASSPHRASE_RETRY:
                return <PassphraseAuth {...authPageProps} isRetry={true}/>;
            case RESP_SSH_OPEN_STEP_TYPE.AUTH_KEY_PAIR_INPUT:
                return <KeyPairAuth {...authPageProps} isRetry={false}/>;
            case RESP_SSH_OPEN_STEP_TYPE.AUTH_KEY_PAIR_RETRY:
                return <KeyPairAuth {...authPageProps} isRetry={true}/>;
            case RESP_SSH_OPEN_STEP_TYPE.AUTH_KEYBOARD_INTERACTIVE_INPUT:
                return <KeyboardInteractiveAuth {...authPageProps} isRetry={false}/>;
            case RESP_SSH_OPEN_STEP_TYPE.AUTH_KEYBOARD_INTERACTIVE_RETRY:
                return <KeyboardInteractiveAuth {...authPageProps} isRetry={true}/>;
            case RESP_SSH_OPEN_STEP_TYPE.AUTH_HOST_FINGERPRINT_CONFIRM:
                return <KnownHost respSocketContext={respSocketContext} handleClose={handleClose} onAnswer={(session_id, event) => sshService.authAnswer(session_id, event)}/>;
            case RESP_SSH_OPEN_STEP_TYPE.AUTH_SSH_AGENT_KEY_SELECT:
                return <SshAgentKeySelect {...authPageProps}/>;
            case RESP_SSH_OPEN_STEP_TYPE.AUTH_METHOD_SELECT:
                return <MethodSelect {...authPageProps}/>;
            default:
                return null;
        }
    };

    return <AuthDialog sessionId={sessionId} reqDevice={reqDevice} connectService={connectService} renderStep={renderStep}/>;
}
