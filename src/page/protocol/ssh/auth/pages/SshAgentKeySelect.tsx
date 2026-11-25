import {Key} from "@mui/icons-material";
import {useI18n} from '@/common/i18n/useI18n.ts';
import {isNotNil} from "@/common/util/StringUtil.ts";
import {usePaneContext} from "@/common/components/tab";
import {Dispatch, SetStateAction, useState} from "react";
import type {ReqDevice} from "@/common/model/req/ReqDevice.ts";
import {createEmptyConfigSsh} from "@/common/model/ConfigSsh.ts";
import {Box, Button, ButtonBase, Typography} from "@mui/material";
import {AgentKeyPrompt} from "@/common/model/dto/SshAuthPrompt.ts";
import {RespSocketContext} from "@/common/model/resp/RespWsContext.ts";
import {OpenStep} from "@/common/model/resp/event/RespEventOpenStep.ts";
import {SSH_AUTH_ANSWER, SshAuthAnswerEvent} from "@/common/model/req/event/WsSsh.ts";
import {AuthLoginUsername} from "@/page/protocol/common/auth/pages/AuthLoginUsername.tsx";
import {useDeviceService, useSshService, useTabManagerService} from "@/common/context/useAppHooks.tsx";

export interface SshAgentKeySelectProps {
    reqDevice: ReqDevice;
    handleClose: () => void;
    respSocketContext: RespSocketContext;
    setAlertInfo: Dispatch<SetStateAction<{ showAlert: boolean, message: string | null }>>
}

export function SshAgentKeySelect(props: SshAgentKeySelectProps) {
    const {t, tf} = useI18n();
    const {paneId} = usePaneContext();
    const sshService = useSshService();
    const deviceService = useDeviceService();
    const tabManagerService = useTabManagerService();
    const {respSocketContext, handleClose, setAlertInfo} = props;
    const [selected, setSelected] = useState<string | null>(null);
    const [agentKeyPrompt] = useState<AgentKeyPrompt>((respSocketContext.payload as OpenStep).data as AgentKeyPrompt);

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
                    request_id: agentKeyPrompt.request_id,
                    code: SSH_AUTH_ANSWER.AGENT_KEY_SELECT_UPDATED,
                }
            }
            sshService.authAnswer(paneId, event)
        }).catch((error) => {
            setAlertInfo({showAlert: true, message: error.message});
        });
    }

    const setAgentKeyFromSelection = (reqDevice: ReqDevice | null): void => {
        if (!reqDevice) {
            setAlertInfo({showAlert: true, message: t('protocol.auth.deviceLoadError')});
            return
        }
        if (!isNotNil(selected)) {
            setAlertInfo({showAlert: true, message: t('protocol.auth.selectAgentKeyAlert')});
            return
        }
        let config_ssh = reqDevice.config_ssh;
        if (!config_ssh) {
            config_ssh = createEmptyConfigSsh();
            reqDevice.config_ssh = config_ssh;
        }
        config_ssh.agent_key_id = selected;
        saveOrUpdateWithConfig(reqDevice);
    }

    const handleSubmit = async (): Promise<void> => {
        deviceService.selectWithConfig(props.reqDevice.device.id).then((reqDevice) => {
            setAgentKeyFromSelection(reqDevice);
        }).catch((error) => {
            setAlertInfo({showAlert: true, message: error.message});
        });
    };

    const resolveOptionClassName = (keyId: string): string => {
        if (selected === keyId) {
            return "tm-protocol-auth-method-option is-selected";
        }
        return "tm-protocol-auth-method-option";
    };

    const resolveAgentTypeLabel = (): string => {
        if (agentKeyPrompt.agent_type === "pageant") {
            return "Pageant";
        }
        return "OpenSSH Agent";
    };

    return (
        <Box key={agentKeyPrompt.request_id} sx={{width: "100%", minWidth: 0, display: "flex", flexDirection: "column", bgcolor: "var(--tm-bg-app)"}}>
            <Box sx={{px: 2.5, pt: 2, pb: 1.25}}>
                <Typography sx={{fontSize: 15, fontWeight: 700, color: "var(--tm-text-primary)", letterSpacing: 0.1}}>
                    {t('protocol.auth.selectAgentKey')}
                </Typography>
                <Typography sx={{fontSize: 12, color: "var(--tm-text-secondary)", mt: 0.75}}>
                    {tf('protocol.auth.selectAgentKeyHint', {agent: resolveAgentTypeLabel()})}
                </Typography>
            </Box>

            <Box sx={{px: 2.5, pb: 1.5}}>
                <Box sx={{display: "flex", flexDirection: "column", gap: 1.25}}>
                    <AuthLoginUsername deviceId={props.reqDevice.device.id}/>
                    <Box sx={{display: "flex", flexDirection: "column", gap: 0.75}}>
                    {agentKeyPrompt.keys.map((entry) => (
                        <ButtonBase key={entry.key_id} onClick={() => setSelected(entry.key_id)} className={resolveOptionClassName(entry.key_id)}>
                            <Box sx={{width: 24, display: "flex", alignItems: "center", justifyContent: "center", flexShrink: 0, color: "var(--tm-text-primary)"}}>
                                <Key sx={{fontSize: 16}}/>
                            </Box>
                            <Box sx={{flex: 1, minWidth: 0, display: "flex", flexDirection: "column", gap: 0.25}}>
                                <Typography sx={{fontSize: 13, fontWeight: 600, color: "var(--tm-text-primary)"}}>
                                    {entry.comment}
                                </Typography>
                                <Typography sx={{fontSize: 11, color: "var(--tm-text-secondary)"}}>
                                    {entry.algorithm} · {entry.fingerprint}
                                </Typography>
                            </Box>
                        </ButtonBase>
                    ))}
                    </Box>
                </Box>
            </Box>

            <Box className="tm-protocol-select-actions" sx={{px: 1.25, py: 0.75, mt: 0, minWidth: 0, borderTop: "1px solid var(--tm-border-subtle)"}}>
                <Button disableRipple onClick={handleCancel} className="tm-btn tm-btn-secondary">{t('common.cancel')}</Button>
                <Box className="tm-protocol-select-actions-left" sx={{minWidth: 0}}>
                    <Button disableRipple onClick={handleSubmit} disabled={!isNotNil(selected)} className="tm-btn tm-btn-primary">{t('common.confirm')}</Button>
                </Box>
            </Box>
        </Box>
    );
}
