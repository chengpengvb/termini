import {createPortal} from 'react-dom';
import {useEffect, useState} from 'react';
import {invoke} from '@tauri-apps/api/core';
import {listen} from '@tauri-apps/api/event';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {inTauri} from '@/common/util/TauriUtil.ts';
import {Box, Button, Typography} from '@mui/material';
import {ReportProblemOutlined} from '@mui/icons-material';
import {reportError} from '@/common/util/ErrorReportUtil.ts';
import {EditField} from '@/page/common/components/input/EditField.tsx';
import {FATAL_EVENT, FATAL_KIND, type RespFatalError} from '@/common/model/resp/RespFatalError.ts';

export function FatalPage() {
    const {t} = useI18n();
    const [note, setNote] = useState<string>('');
    const [phrase, setPhrase] = useState<string>('');
    const [busy, setBusy] = useState<boolean>(false);
    const [fatal, setFatal] = useState<RespFatalError | null>(null);

    const isCrypto = fatal !== null && fatal.kind === FATAL_KIND.CRYPTO;

    const handleCopy = (): void => {
        if (fatal === null) {
            return;
        }
        navigator.clipboard.writeText(fatal.message).then(() => {
            setNote(t('fatal.copied'));
        }).catch(() => {
            setNote(t('fatal.copyFailed'));
        });
    };

    const handleRestart = (): void => {
        setBusy(true);
        void invoke('restart_app');
    };

    const handleRecover = (): void => {
        if (phrase.trim().length === 0 || busy) {
            return;
        }
        setNote('');
        setBusy(true);
        invoke('recover_master_key', {phrase: phrase.trim()}).catch((error: unknown) => {
            setNote(String(error));
            setBusy(false);
        });
    };

    useEffect(() => {
        if (!inTauri()) {
            return;
        }
        invoke<RespFatalError | null>('fatal_error').then(setFatal).catch(() => setFatal(null));
        const unlisten = listen<RespFatalError>(FATAL_EVENT, event => setFatal(event.payload));
        return () => {
            void unlisten.then(off => off());
        };
    }, []);

    useEffect(() => {
        if (fatal === null) {
            return;
        }
        reportError('boot', fatal.message, null);
    }, [fatal]);

    if (fatal === null) {
        return null;
    }
    return createPortal(
        <Box className="tm-fatal-wall">
            <Box className="tm-fatal-card">
                <Box className="tm-fatal-head">
                    <ReportProblemOutlined className="tm-fatal-icon"/>
                    <Box className="tm-fatal-head-main">
                        <Typography className="tm-fatal-title">{isCrypto ? t('fatal.cryptoTitle') : t('fatal.panicTitle')}</Typography>
                        <Typography className="tm-fatal-desc">{isCrypto ? t('fatal.cryptoDesc') : t('fatal.panicDesc')}</Typography>
                    </Box>
                </Box>
                <Box className="tm-fatal-detail">{fatal.message}</Box>
                {isCrypto ? (
                    <Box className="tm-fatal-field">
                        <EditField value={phrase} onChange={setPhrase} placeholder={t('fatal.phrasePlaceholder')}/>
                    </Box>
                ) : null}
                <Box className="tm-fatal-foot">
                    <Typography className="tm-fatal-note">{note}</Typography>
                    <Button disableRipple className="tm-btn tm-btn-secondary" onClick={handleCopy}>{t('fatal.copy')}</Button>
                    {isCrypto ? (
                        <Button disableRipple className="tm-btn tm-btn-primary" disabled={phrase.trim().length === 0 || busy} onClick={handleRecover}>{t('fatal.recover')}</Button>
                    ) : (
                        <Button disableRipple className="tm-btn tm-btn-primary" disabled={busy} onClick={handleRestart}>{t('fatal.restart')}</Button>
                    )}
                </Box>
            </Box>
        </Box>,
        document.body,
    );
}
