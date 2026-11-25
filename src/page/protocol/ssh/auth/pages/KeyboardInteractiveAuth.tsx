import {useI18n} from '@/common/i18n/useI18n.ts';
import {Box, Button, Typography} from "@mui/material";
import {usePaneContext} from "@/common/components/tab";
import {Dispatch, SetStateAction, useState} from "react";
import type {ReqDevice} from "@/common/model/req/ReqDevice.ts";
import {isEmpty, isNotEmpty} from "@/common/util/StringUtil.ts";
import {createEmptyConfigSsh} from "@/common/model/ConfigSsh.ts";
import {InteractivePrompt} from "@/common/model/dto/SshAuthPrompt.ts";
import {EditField} from "@/page/common/components/input/EditField.tsx";
import {RespSocketContext} from "@/common/model/resp/RespWsContext.ts";
import {OpenStep} from "@/common/model/resp/event/RespEventOpenStep.ts";
import {SSH_AUTH_ANSWER, SshAuthAnswerEvent} from "@/common/model/req/event/WsSsh.ts";
import {AuthLoginUsername} from "@/page/protocol/common/auth/pages/AuthLoginUsername.tsx";
import {useDeviceService, useSshService, useTabManagerService} from "@/common/context/useAppHooks.tsx";

export interface KeyboardInteractiveAuthProps {
    isRetry: boolean;
    reqDevice: ReqDevice;
    handleClose: () => void;
    respSocketContext: RespSocketContext;
    setAlertInfo: Dispatch<SetStateAction<{ showAlert: boolean, message: string | null }>>;
}

export function KeyboardInteractiveAuth(props: KeyboardInteractiveAuthProps) {
    const {t, tf} = useI18n();
    const {paneId} = usePaneContext();
    const sshService = useSshService();
    const deviceService = useDeviceService();
    const tabManagerService = useTabManagerService();
    const {respSocketContext, handleClose, setAlertInfo, isRetry} = props;
    const [interactivePrompt] = useState<InteractivePrompt>((respSocketContext.payload as OpenStep).data as InteractivePrompt);
    const [answers, setAnswers] = useState<string[]>(() => interactivePrompt.questions.map(() => String()));

    const resolveTitle = (): string => {
        if (isRetry) {
            return t('protocol.auth.keyboardInteractiveFailed');
        }
        if (isNotEmpty(interactivePrompt.name)) {
            return interactivePrompt.name;
        }
        return t('protocol.auth.keyboardInteractive');
    };

    const resolveHint = (): string | null => {
        if (isRetry) {
            return t('protocol.auth.refillAnswers');
        }
        return null;
    };

    const hint = resolveHint();

    const handleCancel = (): void => {
        handleClose()
        tabManagerService.closePane(paneId);
    };

    const resolveAllAnswered = (): boolean => {
        return answers.every((answer) => isNotEmpty(answer));
    };

    const setAnswerAt = (index: number, value: string): void => {
        setAnswers((prev) => {
            const next = [...prev];
            next[index] = value;
            return next;
        });
    };

    const saveOrUpdateWithConfig = (reqDevice: ReqDevice): void => {
        deviceService.saveOrUpdateWithConfig(reqDevice).then(saved => {
            if (!saved) {
                return;
            }
            handleClose();
            let event: SshAuthAnswerEvent = {
                data: {
                    request_id: interactivePrompt.request_id,
                    code: SSH_AUTH_ANSWER.KEYBOARD_INTERACTIVE_UPDATED,
                }
            }
            sshService.authAnswer(paneId, event)
        }).catch((error) => {
            setAlertInfo({showAlert: true, message: error.message});
        });
    }

    const setAnswersFromInput = (reqDevice: ReqDevice | null): void => {
        if (!reqDevice) {
            setAlertInfo({showAlert: true, message: t('protocol.auth.deviceLoadError')});
            return
        }
        if (!resolveAllAnswered()) {
            setAlertInfo({showAlert: true, message: t('protocol.auth.fillAllAnswers')});
            return
        }
        let config_ssh = reqDevice.config_ssh;
        if (!config_ssh) {
            config_ssh = createEmptyConfigSsh();
            reqDevice.config_ssh = config_ssh;
        }
        config_ssh.keyboard_interactive_answers = [...answers];
        saveOrUpdateWithConfig(reqDevice);
    }

    const handleSubmit = async (): Promise<void> => {
        deviceService.selectWithConfig(props.reqDevice.device.id).then((reqDevice) => {
            setAnswersFromInput(reqDevice);
        }).catch((error) => {
            setAlertInfo({showAlert: true, message: error.message});
        });
    };

    const resolveQuestionLabel = (text: string, index: number): string => {
        if (isEmpty(text)) {
            return tf('protocol.auth.answerLabel', {n: index + 1});
        }
        return text;
    };

    return (
        <Box key={interactivePrompt.request_id} sx={{width: "100%", minWidth: 0, display: "flex", flexDirection: "column", bgcolor: "var(--tm-bg-app)"}}>
            <Box sx={{px: 2.5, pt: 2, pb: 1.25}}>
                <Typography sx={{fontSize: 15, fontWeight: 700, color: "var(--tm-text-primary)", letterSpacing: 0.1}}>
                    {resolveTitle()}
                </Typography>
                {isNotEmpty(interactivePrompt.instruction) ? (
                    <Typography sx={{fontSize: 12, color: "var(--tm-text-secondary)", mt: 0.75, lineHeight: 1.6}}>
                        {interactivePrompt.instruction}
                    </Typography>
                ) : null}
            </Box>

            <Box sx={{px: 2.5, pb: 1.5}}>
                <Box sx={{display: "flex", flexDirection: "column", gap: 1.25}}>
                    <AuthLoginUsername deviceId={props.reqDevice.device.id}/>
                    {isNotEmpty(hint) ? (
                        <Typography sx={{fontSize: 12, color: "var(--tm-text-secondary)", lineHeight: 1.6}}>
                            {hint}
                        </Typography>
                    ) : null}
                    {interactivePrompt.questions.map((question, index) => (
                        <EditField key={index} label={resolveQuestionLabel(question.text, index)} value={answers[index]} onChange={(value) => setAnswerAt(index, value)} placeholder={t('protocol.auth.answerPlaceholder')} password={!question.echo} />
                    ))}
                </Box>
            </Box>

            <Box className="tm-protocol-select-actions" sx={{px: 1.25, py: 0.75, mt: 0, minWidth: 0, borderTop: "1px solid var(--tm-border-subtle)"}}>
                <Button disableRipple onClick={handleCancel} className="tm-btn tm-btn-secondary">{t('common.cancel')}</Button>
                <Box className="tm-protocol-select-actions-left" sx={{minWidth: 0}}>
                    <Button disableRipple onClick={handleSubmit} disabled={!resolveAllAnswered()} className="tm-btn tm-btn-primary">{t('common.confirm')}</Button>
                </Box>
            </Box>
        </Box>
    );
}
