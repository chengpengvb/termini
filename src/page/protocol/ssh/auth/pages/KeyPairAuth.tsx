import {useI18n} from '@/common/i18n/useI18n.ts';
import {Box, Button, Typography} from "@mui/material";
import {isNotEmpty} from "@/common/util/StringUtil.ts";
import {usePaneContext} from "@/common/components/tab";
import {Dispatch, SetStateAction, useState} from "react";
import type {ReqDevice} from "@/common/model/req/ReqDevice.ts";
import {createEmptyConfigSsh} from "@/common/model/ConfigSsh.ts";
import {RespSocketContext} from "@/common/model/resp/RespWsContext.ts";
import {OpenStep} from "@/common/model/resp/event/RespEventOpenStep.ts";
import {InputPrompt, RetryPrompt} from "@/common/model/dto/SshAuthPrompt.ts";
import {SSH_AUTH_ANSWER, SshAuthAnswerEvent} from "@/common/model/req/event/WsSsh.ts";
import {KeyPairFields} from "@/page/home/devices/edit/section/shared/KeyPairFields.tsx";
import {AuthLoginUsername} from "@/page/protocol/common/auth/pages/AuthLoginUsername.tsx";
import {useDeviceService, useSshService, useTabManagerService} from "@/common/context/useAppHooks.tsx";

export interface KeyPairAuthProps {
    isRetry: boolean;
    reqDevice: ReqDevice;
    handleClose: () => void;
    respSocketContext: RespSocketContext;
    setAlertInfo: Dispatch<SetStateAction<{ showAlert: boolean, message: string | null }>>;
}

export function KeyPairAuth(props: KeyPairAuthProps) {
    const {t} = useI18n();
    const {paneId} = usePaneContext();
    const sshService = useSshService();
    const deviceService = useDeviceService();
    const tabManagerService = useTabManagerService();
    const [keyPairId, setKeyPairId] = useState<string>(String());
    const {respSocketContext, handleClose, setAlertInfo, isRetry} = props;
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
            return t('protocol.auth.publicKeyAuthFailed');
        }
        return t('protocol.auth.selectKey');
    };

    const resolveSubtitle = (): string => {
        if (isRetry) {
            return t('protocol.auth.sshPublicKeyNeedValid');
        }
        return t('protocol.auth.sshPublicKeyNeed');
    };

    const resolveHint = (): string => {
        if (isRetry) {
            return t('protocol.auth.reselectKey');
        }
        return t('protocol.auth.keyNotConfigured');
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
                    code: SSH_AUTH_ANSWER.KEY_PAIR_UPDATED,
                }
            }
            sshService.authAnswer(paneId, event)
        }).catch((error) => {
            setAlertInfo({showAlert: true, message: error.message});
        });
    }

    const setKeyPairFromSelection = (reqDevice: ReqDevice | null): void => {
        if (!reqDevice) {
            setAlertInfo({showAlert: true, message: t('protocol.auth.deviceLoadError')});
            return
        }
        if (!isNotEmpty(keyPairId)) {
            setAlertInfo({showAlert: true, message: t('protocol.auth.selectKeyAlert')});
            return
        }
        let config_ssh = reqDevice.config_ssh;
        if (!config_ssh) {
            config_ssh = createEmptyConfigSsh();
            reqDevice.config_ssh = config_ssh;
        }
        config_ssh.config_key_pair_id = keyPairId;
        saveOrUpdateWithConfig(reqDevice);
    }

    const handleSubmit = async (): Promise<void> => {
        deviceService.selectWithConfig(props.reqDevice.device.id).then((reqDevice) => {
            setKeyPairFromSelection(reqDevice);
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
                    <AuthLoginUsername deviceId={props.reqDevice.device.id}/>
                    {isNotEmpty(lastError) ? (
                        <Typography sx={{fontSize: 12, color: "var(--tm-status-danger)", lineHeight: 1.6}}>
                            {lastError}
                        </Typography>
                    ) : null}
                    <Typography sx={{fontSize: 12, color: "var(--tm-text-secondary)", lineHeight: 1.6}}>
                        {resolveHint()}
                    </Typography>
                    <KeyPairFields value={keyPairId} onChange={setKeyPairId}/>
                </Box>
            </Box>

            <Box className="tm-protocol-select-actions" sx={{px: 1.25, py: 0.75, mt: 0, minWidth: 0, borderTop: "1px solid var(--tm-border-subtle)"}}>
                <Button disableRipple onClick={handleCancel} className="tm-btn tm-btn-secondary">{t('common.cancel')}</Button>
                <Box className="tm-protocol-select-actions-left" sx={{minWidth: 0}}>
                    <Button disableRipple onClick={handleSubmit} disabled={!isNotEmpty(keyPairId)} className="tm-btn tm-btn-primary">{t('common.confirm')}</Button>
                </Box>
            </Box>
        </Box>
    );
}
