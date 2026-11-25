import {useState} from "react";
import {useI18n} from '@/common/i18n/useI18n.ts';
import {Box, Button, Typography} from "@mui/material";
import {usePaneContext} from "@/common/components/tab";
import {useTabManagerService} from "@/common/context/useAppHooks.tsx";
import {RespSocketContext} from "@/common/model/resp/RespWsContext.ts";
import {OpenStep} from "@/common/model/resp/event/RespEventOpenStep.ts";
import {SSH_AUTH_ANSWER, SshAuthAnswerEvent} from "@/common/model/req/event/WsSsh.ts";
import {FingerprintPrompt, SSH_FINGERPRINT_REASON} from "@/common/model/dto/SshAuthPrompt.ts";

export interface KnownHostProps {
    handleClose: () => void;
    respSocketContext: RespSocketContext;
    onAnswer: (session_id: string, event: SshAuthAnswerEvent) => void;
}

export function KnownHost(props: KnownHostProps) {
    const {t} = useI18n();
    const {paneId} = usePaneContext();
    const tabManagerService = useTabManagerService();
    const {respSocketContext, handleClose, onAnswer} = props;
    const [fingerprintPrompt] = useState<FingerprintPrompt>((respSocketContext.payload as OpenStep).data as FingerprintPrompt);

    const handleReject = () => {
        handleClose();
        let event: SshAuthAnswerEvent = {
            data: {
                request_id: fingerprintPrompt.request_id,
                code: SSH_AUTH_ANSWER.HOST_FINGERPRINT_REJECT,
            }
        }
        onAnswer(paneId, event)
        tabManagerService.closePane(paneId);
    }

    const handleTrustOnce = () => {
        handleClose();
        let event: SshAuthAnswerEvent = {
            data: {
                request_id: fingerprintPrompt.request_id,
                code: SSH_AUTH_ANSWER.HOST_FINGERPRINT_TRUST_ONCE,
            }
        }
        onAnswer(paneId, event)

    }

    const handleTrustForever = () => {
        handleClose();
        let event: SshAuthAnswerEvent = {
            data: {
                request_id: fingerprintPrompt.request_id,
                code: SSH_AUTH_ANSWER.HOST_FINGERPRINT_TRUST_ALWAYS,
            }
        }
        onAnswer(paneId, event)
    }

    const isChanged = fingerprintPrompt.reason === SSH_FINGERPRINT_REASON.CHANGED;

    const renderFingerprintBox = (algorithm: string, sha256: string) => (
        <Box sx={{display: "flex", flexDirection: "column", gap: 0.75, p: 1.25, width: "100%", minWidth: 0, bgcolor: "var(--tm-bg-button)", borderRadius: "6px", border: "1px solid var(--tm-border-subtle)", boxSizing: "border-box"}}>
            <Box sx={{display: "flex", gap: 1}}>
                <Typography sx={{fontSize: 12, color: "var(--tm-text-secondary)", minWidth: 56}}>{t('common.algorithm')}</Typography>
                <Typography sx={{fontSize: 12, color: "var(--tm-text-primary)", flex: 1, minWidth: 0, wordBreak: "break-all"}}>{algorithm}</Typography>
            </Box>
            <Box sx={{display: "flex", gap: 1}}>
                <Typography sx={{fontSize: 12, color: "var(--tm-text-secondary)", minWidth: 56}}>SHA256</Typography>
                <Typography sx={{fontSize: 12, color: "var(--tm-text-primary)", flex: 1, minWidth: 0, fontFamily: '"JetBrains Mono", "Fira Code", Consolas, monospace', wordBreak: "break-all"}}>{sha256}</Typography>
            </Box>
        </Box>
    );

    return (
        <Box sx={{width: "100%", minWidth: 0, display: "flex", flexDirection: "column", bgcolor: "var(--tm-bg-app)"}}>
            <Box sx={{px: 2.5, pt: 2, pb: 1.25}}>
                <Typography sx={{fontSize: 15, fontWeight: 700, color: isChanged ? "var(--tm-status-danger)" : "var(--tm-text-primary)", letterSpacing: 0.1}}>
                    {isChanged ? t('protocol.auth.knownHostChangedTitle') : t('protocol.auth.knownHostFirstTitle')}
                </Typography>
                <Typography sx={{fontSize: 12, color: "var(--tm-text-secondary)", mt: 0.75}}>
                    {fingerprintPrompt.host}:{fingerprintPrompt.port}
                </Typography>
            </Box>

            <Box sx={{px: 2.5, pb: 1.5}}>
                {isChanged && fingerprintPrompt.previous_sha256 ? (
                    <Box sx={{display: "flex", flexDirection: "column", gap: 1.25}}>
                        <Typography sx={{fontSize: 12, color: "var(--tm-status-danger)", lineHeight: 1.6}}>
                            {t('protocol.auth.knownHostChangedWarning')}
                        </Typography>
                        <Box sx={{display: "flex", flexDirection: "column", gap: 1}}>
                            <Typography sx={{fontSize: 11, fontWeight: 700, color: "var(--tm-text-secondary)", letterSpacing: 0.8, textTransform: "uppercase"}}>{t('protocol.auth.knownHostSaved')}</Typography>
                            {renderFingerprintBox(fingerprintPrompt.algorithm, fingerprintPrompt.previous_sha256)}
                            <Typography sx={{fontSize: 11, fontWeight: 700, color: "var(--tm-text-secondary)", letterSpacing: 0.8, textTransform: "uppercase"}}>{t('protocol.auth.knownHostCurrent')}</Typography>
                            {renderFingerprintBox(fingerprintPrompt.algorithm, fingerprintPrompt.sha256)}
                        </Box>
                        <Typography sx={{fontSize: 12, color: "var(--tm-text-secondary)"}}>{t('protocol.auth.knownHostTrustOverwrite')}</Typography>
                    </Box>
                ) : (
                    <Box sx={{display: "flex", flexDirection: "column", gap: 1.25}}>
                        {renderFingerprintBox(fingerprintPrompt.algorithm, fingerprintPrompt.sha256)}
                        <Typography sx={{fontSize: 12, color: "var(--tm-text-secondary)"}}>{t('protocol.auth.knownHostVerifyFingerprint')}</Typography>
                    </Box>
                )}
            </Box>

            <Box className="tm-protocol-select-actions" sx={{px: 1.25, py: 0.75, mt: 0, minWidth: 0, borderTop: "1px solid var(--tm-border-subtle)"}}>
                <Button disableRipple onClick={handleReject} className="tm-btn tm-btn-secondary">{t('common.reject')}</Button>
                <Box className="tm-protocol-select-actions-left" sx={{minWidth: 0}}>
                    <Button disableRipple onClick={handleTrustOnce} className="tm-btn tm-btn-secondary">{t('protocol.auth.trustOnce')}</Button>
                    <Button disableRipple onClick={handleTrustForever} className="tm-btn tm-btn-primary">{t('protocol.auth.trustForever')}</Button>
                </Box>
            </Box>
        </Box>
    );
}
