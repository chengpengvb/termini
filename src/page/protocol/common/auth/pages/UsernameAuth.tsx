import {PROTOCOL} from "@/common/const/Const.ts";
import {useI18n} from '@/common/i18n/useI18n.ts';
import {Box, Button, Typography} from "@mui/material";
import {isNotEmpty} from "@/common/util/StringUtil.ts";
import {usePaneContext} from "@/common/components/tab";
import {Dispatch, SetStateAction, useState} from "react";
import type {ReqDevice} from "@/common/model/req/ReqDevice.ts";
import {createEmptyConfigSsh} from "@/common/model/ConfigSsh.ts";
import type {ConfigTermBase} from "@/common/model/ConfigTermBase.ts";
import {EditField} from "@/page/common/components/input/EditField.tsx";
import {RespSocketContext} from "@/common/model/resp/RespWsContext.ts";
import {OpenStep} from "@/common/model/resp/event/RespEventOpenStep.ts";
import {createEmptyConfigTermBase} from "@/common/model/ConfigTermBase.ts";
import {InputPrompt, RetryPrompt} from "@/common/model/dto/SshAuthPrompt.ts";
import {SSH_AUTH_ANSWER, SshAuthAnswerEvent} from "@/common/model/req/event/WsSsh.ts";
import {useDeviceService, useRdpService, useSshService, useTabManagerService} from "@/common/context/useAppHooks.tsx";

export interface UsernameAuthProps {
    isRetry: boolean;
    reqDevice: ReqDevice;
    handleClose: () => void;
    protocol?: number;
    respSocketContext: RespSocketContext;
    setAlertInfo: Dispatch<SetStateAction<{ showAlert: boolean, message: string | null }>>;
}

export function UsernameAuth(props: UsernameAuthProps) {
    const {t, tf} = useI18n();
    const {paneId} = usePaneContext();
    const sshService = useSshService();
    const rdpService = useRdpService();
    const deviceService = useDeviceService();
    const tabManagerService = useTabManagerService();
    const [username, setUsername] = useState<string>(String());
    const {respSocketContext, handleClose, setAlertInfo, isRetry} = props;
    const protocol = props.protocol ?? PROTOCOL.SSH;
    const service = protocol === PROTOCOL.RDP ? rdpService : sshService;
    const protocolLabel = protocol === PROTOCOL.RDP ? "RDP" : "SSH";
    const [prompt] = useState<InputPrompt | RetryPrompt>((respSocketContext.payload as OpenStep).data as InputPrompt | RetryPrompt);

    const resolveLastError = (): string | null => {
        if (!isRetry) {
            return null;
        }
        return (prompt as RetryPrompt).last_error;
    };

    const lastError = resolveLastError();

    const resolveTitle = (): string => {
        if (isRetry) {
            return t('protocol.auth.usernameAuthFailed');
        }
        return t('protocol.auth.enterUsername');
    };

    const resolveSubtitle = (): string => {
        if (isRetry) {
            return tf('protocol.auth.usernameAuthNeedValid', {protocol: protocolLabel});
        }
        return tf('protocol.auth.usernameAuthNeed', {protocol: protocolLabel});
    };

    const resolveHint = (): string => {
        if (isRetry) {
            return t('protocol.auth.refillUsername');
        }
        return t('protocol.auth.usernameNotConfigured');
    };

    const handleCancel = (): void => {
        handleClose()
        tabManagerService.closePane(paneId);
    };

    const saveOrUpdateWithConfig = (reqDevice: ReqDevice): void => {
        deviceService.saveOrUpdateWithConfig(reqDevice).then(saved => {
            if (!saved) {
                return;
            }
            handleClose();
            let event: SshAuthAnswerEvent = {
                data: {
                    request_id: prompt.request_id,
                    code: SSH_AUTH_ANSWER.USERNAME_UPDATED,
                }
            }
            service.authAnswer(paneId, event)
        }).catch((error) => {
            setAlertInfo({showAlert: true, message: error.message});
        });
    }

    const resolveBase = (reqDevice: ReqDevice): ConfigTermBase | null => {
        if (protocol === PROTOCOL.RDP) {
            const config_rdp = reqDevice.config_rdp;
            if (!config_rdp) {
                setAlertInfo({showAlert: true, message: t('protocol.auth.rdpConfigMissing')});
                return null;
            }
            if (!config_rdp.base) {
                config_rdp.base = createEmptyConfigTermBase();
            }
            return config_rdp.base;
        }
        let config_ssh = reqDevice.config_ssh;
        if (!config_ssh) {
            config_ssh = createEmptyConfigSsh();
            reqDevice.config_ssh = config_ssh;
        }
        if (!config_ssh.base) {
            config_ssh.base = createEmptyConfigTermBase();
        }
        return config_ssh.base;
    }

    const setUsernameFromInput = (reqDevice: ReqDevice | null): void => {
        if (!reqDevice) {
            setAlertInfo({showAlert: true, message: t('protocol.auth.deviceLoadError')});
            return
        }
        const base = resolveBase(reqDevice);
        if (!base) {
            return
        }
        base.username = username;
        saveOrUpdateWithConfig(reqDevice);
    }

    const handleSubmit = async (): Promise<void> => {
        deviceService.selectWithConfig(props.reqDevice.device.id).then((reqDevice) => {
            setUsernameFromInput(reqDevice);
        }).catch((error) => {
            setAlertInfo({showAlert: true, message: error.message});
        });
    };

    return (
        <Box key={prompt.request_id} sx={{width: "100%", minWidth: 0, display: "flex", flexDirection: "column", bgcolor: "var(--tm-bg-app)"}}>
            <Box sx={{px: 2.5, pt: 2, pb: 1.25}}>
                <Typography sx={{fontSize: 15, fontWeight: 700, color: "var(--tm-text-primary)", letterSpacing: 0.1}}>
                    {resolveTitle()}
                </Typography>
                <Typography sx={{fontSize: 12, color: "var(--tm-text-secondary)", mt: 0.75}}>
                    {resolveSubtitle()}
                </Typography>
            </Box>

            <Box sx={{px: 2.5, pb: 1.5}}>
                <Box sx={{display: "flex", flexDirection: "column", gap: 1.25}}>
                    {isNotEmpty(lastError) ? (
                        <Typography sx={{fontSize: 12, color: "var(--tm-status-danger)", lineHeight: 1.6}}>
                            {lastError}
                        </Typography>
                    ) : null}
                    <Typography sx={{fontSize: 12, color: "var(--tm-text-secondary)", lineHeight: 1.6}}>
                        {resolveHint()}
                    </Typography>
                    <EditField label={t('home.common.username')} value={username} onChange={setUsername} placeholder={t('protocol.auth.usernamePlaceholder')}/>
                </Box>
            </Box>

            <Box className="tm-protocol-select-actions" sx={{px: 1.25, py: 0.75, mt: 0, minWidth: 0, borderTop: "1px solid var(--tm-border-subtle)"}}>
                <Button disableRipple onClick={handleCancel} className="tm-btn tm-btn-secondary">{t('common.cancel')}</Button>
                <Box className="tm-protocol-select-actions-left" sx={{minWidth: 0}}>
                    <Button disableRipple onClick={handleSubmit} className="tm-btn tm-btn-primary">{t('common.confirm')}</Button>
                </Box>
            </Box>
        </Box>
    );
}
