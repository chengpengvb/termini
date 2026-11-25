import {RESP_SSH_OPEN_STEP_TYPE} from "@/common/model/resp/event/RespEventSshAuth.ts";

export const AUTO_TYPE_MAP: Set<number> = new Set([
    RESP_SSH_OPEN_STEP_TYPE.AUTH_USERNAME_INPUT,
    RESP_SSH_OPEN_STEP_TYPE.AUTH_USERNAME_RETRY,
    RESP_SSH_OPEN_STEP_TYPE.AUTH_PASSWORD_INPUT,
    RESP_SSH_OPEN_STEP_TYPE.AUTH_PASSWORD_RETRY,
    RESP_SSH_OPEN_STEP_TYPE.AUTH_PASSPHRASE_INPUT,
    RESP_SSH_OPEN_STEP_TYPE.AUTH_PASSPHRASE_RETRY,
    RESP_SSH_OPEN_STEP_TYPE.AUTH_KEY_PAIR_INPUT,
    RESP_SSH_OPEN_STEP_TYPE.AUTH_KEY_PAIR_RETRY,
    RESP_SSH_OPEN_STEP_TYPE.AUTH_KEYBOARD_INTERACTIVE_INPUT,
    RESP_SSH_OPEN_STEP_TYPE.AUTH_KEYBOARD_INTERACTIVE_RETRY,
    RESP_SSH_OPEN_STEP_TYPE.AUTH_HOST_FINGERPRINT_CONFIRM,
    RESP_SSH_OPEN_STEP_TYPE.AUTH_SSH_AGENT_KEY_SELECT,
    RESP_SSH_OPEN_STEP_TYPE.AUTH_METHOD_SELECT,
]);

export const STEP_TYPE_MESSAGE_MAP: Record<number, string> = {
    [RESP_SSH_OPEN_STEP_TYPE.PROBE_START]: 'protocol.openStep.ssh.probeStart',
    [RESP_SSH_OPEN_STEP_TYPE.PROBE_TCP_CONNECTING]: 'protocol.openStep.ssh.probeTcpConnecting',
    [RESP_SSH_OPEN_STEP_TYPE.PROBE_TCP_CONNECTED]: 'protocol.openStep.ssh.probeTcpConnected',
    [RESP_SSH_OPEN_STEP_TYPE.PROBE_NEGOTIATING]: 'protocol.openStep.ssh.probeNegotiating',
    [RESP_SSH_OPEN_STEP_TYPE.PROBE_COMPLETED]: 'protocol.openStep.ssh.probeCompleted',
    [RESP_SSH_OPEN_STEP_TYPE.AUTH_START]: 'protocol.openStep.ssh.authStart',

    [RESP_SSH_OPEN_STEP_TYPE.AUTH_USERNAME_INPUT]: 'protocol.openStep.ssh.authUsernameInput',
    [RESP_SSH_OPEN_STEP_TYPE.AUTH_USERNAME_RETRY]: 'protocol.openStep.ssh.authUsernameRetry',
    [RESP_SSH_OPEN_STEP_TYPE.AUTH_PASSWORD_INPUT]: 'protocol.openStep.ssh.authPasswordInput',
    [RESP_SSH_OPEN_STEP_TYPE.AUTH_PASSWORD_RETRY]: 'protocol.openStep.ssh.authPasswordRetry',
    [RESP_SSH_OPEN_STEP_TYPE.AUTH_PASSPHRASE_INPUT]: 'protocol.openStep.ssh.authPassphraseInput',
    [RESP_SSH_OPEN_STEP_TYPE.AUTH_PASSPHRASE_RETRY]: 'protocol.openStep.ssh.authPassphraseRetry',
    [RESP_SSH_OPEN_STEP_TYPE.AUTH_KEY_PAIR_INPUT]: 'protocol.openStep.ssh.authKeyPairInput',
    [RESP_SSH_OPEN_STEP_TYPE.AUTH_KEY_PAIR_RETRY]: 'protocol.openStep.ssh.authKeyPairRetry',
    [RESP_SSH_OPEN_STEP_TYPE.AUTH_KEYBOARD_INTERACTIVE_INPUT]: 'protocol.openStep.ssh.authKeyboardInteractiveInput',
    [RESP_SSH_OPEN_STEP_TYPE.AUTH_KEYBOARD_INTERACTIVE_RETRY]: 'protocol.openStep.ssh.authKeyboardInteractiveRetry',
    [RESP_SSH_OPEN_STEP_TYPE.AUTH_HOST_FINGERPRINT_CONFIRM]: 'protocol.openStep.ssh.authHostFingerprintConfirm',
    [RESP_SSH_OPEN_STEP_TYPE.AUTH_SSH_AGENT_KEY_SELECT]: 'protocol.openStep.ssh.authSshAgentKeySelect',
    [RESP_SSH_OPEN_STEP_TYPE.AUTH_METHOD_SELECT]: 'protocol.openStep.ssh.authMethodSelect',

    [RESP_SSH_OPEN_STEP_TYPE.SESSION_OPENED]: 'protocol.openStep.ssh.sessionOpened',
    [RESP_SSH_OPEN_STEP_TYPE.ERROR_TCP_FAILED]: 'protocol.openStep.ssh.errorTcpFailed',
    [RESP_SSH_OPEN_STEP_TYPE.ERROR_PROBE_FAILED]: 'protocol.openStep.ssh.errorProbeFailed',
    [RESP_SSH_OPEN_STEP_TYPE.ERROR_AUTH_FAILED]: 'protocol.openStep.ssh.errorAuthFailed',
    [RESP_SSH_OPEN_STEP_TYPE.ERROR_AUTH_TIMEOUT]: 'protocol.openStep.ssh.errorAuthTimeout',
    [RESP_SSH_OPEN_STEP_TYPE.ERROR_CHANNEL_FAILED]: 'protocol.openStep.ssh.errorChannelFailed',
    [RESP_SSH_OPEN_STEP_TYPE.ERROR_OPEN_CANCELLED]: 'protocol.openStep.ssh.errorOpenCancelled',
}