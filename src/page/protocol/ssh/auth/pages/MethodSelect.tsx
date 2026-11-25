import {useI18n} from '@/common/i18n/useI18n.ts';
import {isNotNil} from "@/common/util/StringUtil.ts";
import {usePaneContext} from "@/common/components/tab";
import {Key, Keyboard, Password} from "@mui/icons-material";
import type {ReqDevice} from "@/common/model/req/ReqDevice.ts";
import {createEmptyConfigSsh} from "@/common/model/ConfigSsh.ts";
import {Box, Button, ButtonBase, Typography} from "@mui/material";
import {Dispatch, ReactNode, SetStateAction, useState} from "react";
import {RespSocketContext} from "@/common/model/resp/RespWsContext.ts";
import {OpenStep} from "@/common/model/resp/event/RespEventOpenStep.ts";
import {SSH_AUTH_ANSWER, SshAuthAnswerEvent} from "@/common/model/req/event/WsSsh.ts";
import {AuthLoginUsername} from "@/page/protocol/common/auth/pages/AuthLoginUsername.tsx";
import {MethodSelectPrompt, SSH_AUTH_METHOD_CODE} from "@/common/model/dto/SshAuthPrompt.ts";
import {useDeviceService, useSshService, useTabManagerService} from "@/common/context/useAppHooks.tsx";

export interface MethodSelectProps {
    reqDevice: ReqDevice;
    handleClose: () => void;
    respSocketContext: RespSocketContext;
    setAlertInfo: Dispatch<SetStateAction<{ showAlert: boolean, message: string | null }>>
}

export function MethodSelect(props: MethodSelectProps) {
    const {t} = useI18n();
    const methodIconMap: Record<number, ReactNode> = {
        [SSH_AUTH_METHOD_CODE.PASSWORD]: <Password sx={{fontSize: 16}}/>,
        [SSH_AUTH_METHOD_CODE.PUBLIC_KEY]: <Key sx={{fontSize: 16}}/>,
        [SSH_AUTH_METHOD_CODE.KEYBOARD_INTERACTIVE]: <Keyboard sx={{fontSize: 16}}/>,
    };
    const {paneId} = usePaneContext();
    const sshService = useSshService();
    const deviceService = useDeviceService();
    const tabManagerService = useTabManagerService();
    const {respSocketContext, handleClose, setAlertInfo} = props;
    const [methodPrompt] = useState<MethodSelectPrompt>((respSocketContext.payload as OpenStep).data as MethodSelectPrompt);

    const methodLabelMap: Record<number, string> = {
        [SSH_AUTH_METHOD_CODE.PASSWORD]: t('protocol.auth.methodPassword'),
        [SSH_AUTH_METHOD_CODE.PUBLIC_KEY]: t('protocol.auth.methodPublicKey'),
        [SSH_AUTH_METHOD_CODE.KEYBOARD_INTERACTIVE]: t('protocol.auth.methodKeyboardInteractive'),
    };

    const methodDescMap: Record<number, string> = {
        [SSH_AUTH_METHOD_CODE.PASSWORD]: t('protocol.auth.methodPasswordDesc'),
        [SSH_AUTH_METHOD_CODE.PUBLIC_KEY]: t('protocol.auth.methodPublicKeyDesc'),
        [SSH_AUTH_METHOD_CODE.KEYBOARD_INTERACTIVE]: t('protocol.auth.methodKeyboardInteractiveDesc'),
    };

    const resolveDefaultCode = (): number | null => {
        if (isNotNil(methodPrompt.default) && methodPrompt.methods.includes(methodPrompt.default)) {
            return methodPrompt.default;
        }
        return null;
    };

    const recommendedCode = resolveDefaultCode();
    const [selected, setSelected] = useState<number | null>(() => resolveDefaultCode());

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
                    request_id: methodPrompt.request_id,
                    code: SSH_AUTH_ANSWER.METHOD_SELECT_UPDATED,
                }
            }
            sshService.authAnswer(paneId, event)
        }).catch((error) => {
            setAlertInfo({showAlert: true, message: error.message});
        });
    }

    const setMethodFromSelection = (reqDevice: ReqDevice | null): void => {
        if (!reqDevice) {
            setAlertInfo({showAlert: true, message: t('protocol.auth.deviceLoadError')});
            return
        }
        if (!isNotNil(selected)) {
            setAlertInfo({showAlert: true, message: t('protocol.auth.selectMethodAlert')});
            return
        }
        let config_ssh = reqDevice.config_ssh;
        if (!config_ssh) {
            config_ssh = createEmptyConfigSsh();
            reqDevice.config_ssh = config_ssh;
        }
        config_ssh.preferred_auth_method = selected;
        saveOrUpdateWithConfig(reqDevice);
    }

    const handleSubmit = async (): Promise<void> => {
        deviceService.selectWithConfig(props.reqDevice.device.id).then((reqDevice) => {
            setMethodFromSelection(reqDevice);
        }).catch((error) => {
            setAlertInfo({showAlert: true, message: error.message});
        });
    };

    const resolveOptionClassName = (code: number): string => {
        if (selected === code) {
            return "tm-protocol-auth-method-option is-selected";
        }
        return "tm-protocol-auth-method-option";
    };

    return (
        <Box key={methodPrompt.request_id} sx={{width: "100%", minWidth: 0, display: "flex", flexDirection: "column", bgcolor: "var(--tm-bg-app)"}}>
            <Box sx={{px: 2.5, pt: 2, pb: 1.25}}>
                <Typography sx={{fontSize: 15, fontWeight: 700, color: "var(--tm-text-primary)", letterSpacing: 0.1}}>
                    {t('protocol.auth.selectMethod')}
                </Typography>
                <Typography sx={{fontSize: 12, color: "var(--tm-text-secondary)", mt: 0.75}}>
                    {t('protocol.auth.selectMethodHint')}
                </Typography>
            </Box>

            <Box sx={{px: 2.5, pb: 1.5}}>
                <Box sx={{display: "flex", flexDirection: "column", gap: 1.25}}>
                    <AuthLoginUsername deviceId={props.reqDevice.device.id}/>
                    <Box sx={{display: "flex", flexDirection: "column", gap: 0.75}}>
                    {methodPrompt.methods.map((code) => (
                        <ButtonBase key={code} onClick={() => setSelected(code)} className={resolveOptionClassName(code)}>
                            <Box sx={{width: 24, display: "flex", alignItems: "center", justifyContent: "center", flexShrink: 0, color: "var(--tm-text-primary)"}}>
                                {methodIconMap[code]}
                            </Box>
                            <Box sx={{flex: 1, minWidth: 0, display: "flex", flexDirection: "column", gap: 0.25}}>
                                <Box sx={{display: "flex", alignItems: "center", gap: 0.75, minWidth: 0}}>
                                    <Typography sx={{fontSize: 13, fontWeight: 600, color: "var(--tm-text-primary)"}}>
                                        {methodLabelMap[code]}
                                    </Typography>
                                    {code === recommendedCode ? (
                                        <Typography sx={{fontSize: 10, color: "var(--tm-icon-accent)", border: "1px solid var(--tm-border-accent)", borderRadius: "4px", px: 0.5, lineHeight: 1.4, flexShrink: 0}}>
                                            {t('common.recommend')}
                                        </Typography>
                                    ) : null}
                                </Box>
                                <Typography sx={{fontSize: 11, color: "var(--tm-text-secondary)"}}>
                                    {methodDescMap[code]}
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
