import {useState} from 'react';
import type {ReactNode} from 'react';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {isNotNil} from '@/common/util/StringUtil.ts';
import {Box, Button, Typography} from '@mui/material';
import {useSystemUtilService} from '@/common/context/useAppHooks.tsx';

export function MasterKeySection() {
    const {t} = useI18n();
    const systemUtilService = useSystemUtilService();

    const [note, setNote] = useState<string>('');
    const [phrase, setPhrase] = useState<string>('');
    const [error, setError] = useState<string | null>(null);
    const [loading, setLoading] = useState<boolean>(false);

    const handleToggle = (): void => {
        if (phrase !== '') {
            setNote('');
            setPhrase('');
            return;
        }
        setError(null);
        setLoading(true);
        systemUtilService.getMasterKeyPhrase().then(setPhrase).catch(() => {
            setError(t('set.masterKey.revealFailed'));
        }).finally(() => {
            setLoading(false);
        });
    };

    const handleCopy = (): void => {
        navigator.clipboard.writeText(phrase).then(() => {
            setNote(t('set.masterKey.copied'));
        }).catch(() => {
            setNote(t('set.masterKey.copyFailed'));
        });
    };

    const resolveToggleLabel = (): string => {
        if (loading) {
            return t('set.masterKey.revealing');
        }
        if (phrase !== '') {
            return t('set.masterKey.hide');
        }
        return t('set.masterKey.reveal');
    };

    const renderError = (): ReactNode => {
        if (!isNotNil(error)) {
            return null;
        }
        return <Typography className="tm-set-section-tip is-error">{error}</Typography>;
    };

    return (
        <Box className="tm-set-card">
            <Box className="tm-set-card-head">
                <Box className="tm-set-card-head-main">
                    <Typography className="tm-set-card-title">{t('set.masterKey.title')}</Typography>
                    <Typography className="tm-set-card-desc">{t('set.masterKey.desc')}</Typography>
                </Box>
                <Box className="tm-set-card-head-action">
                    {renderError()}
                </Box>
            </Box>
            <Box className="tm-set-card-body">
                <Box className="tm-set-row">
                    <Box className="tm-set-row-main">
                        <Typography className="tm-set-row-label">{t('set.masterKey.phrase')}</Typography>
                        <Typography className="tm-set-row-hint">{t('set.masterKey.phraseHint')}</Typography>
                    </Box>
                    <Box className="tm-set-row-control">
                        <Button disableRipple fullWidth className="tm-btn tm-btn-secondary" disabled={loading} onClick={handleToggle}>{resolveToggleLabel()}</Button>
                    </Box>
                </Box>
                {phrase !== '' ? (
                    <Box className="tm-set-phrase">
                        <Typography className="tm-set-phrase-text">{phrase}</Typography>
                        <Box className="tm-set-phrase-foot">
                            <Typography className="tm-set-phrase-note">{note === '' ? t('set.masterKey.warn') : note}</Typography>
                            <Button disableRipple className="tm-btn tm-btn-secondary" onClick={handleCopy}>{t('set.masterKey.copy')}</Button>
                        </Box>
                    </Box>
                ) : null}
            </Box>
        </Box>
    );
}
