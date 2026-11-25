import type {ReactNode} from 'react';
import {useEffect, useState} from 'react';
import {Box, Typography} from '@mui/material';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {isNotNil} from '@/common/util/StringUtil.ts';
import {useConfigService} from '@/common/context/useAppHooks.tsx';
import {ModelSelectRow} from '@/page/home/set/common/ModelSelectRow.tsx';

export function AlertSection() {
    const {t} = useI18n();
    const configService = useConfigService();

    const [provideId, setProvideId] = useState<string>('');
    const [error, setError] = useState<string | null>(null);

    const handleModelChange = (next: string): void => {
        setProvideId(next);
        setError(null);
        configService.saveJobAlert(next === '' ? null : next).catch(() => {
            setError(t('common.saveFailed'));
        });
    };

    const renderError = (): ReactNode => {
        if (!isNotNil(error)) {
            return null;
        }
        return <Typography className="tm-set-section-tip is-error">{error}</Typography>;
    };

    const renderModelHint = (): ReactNode => {
        if (provideId === '') {
            return <Typography className="tm-set-row-hint">{t('set.alert.modelHintEmpty')}</Typography>;
        }
        return <Typography className="tm-set-row-hint">{t('set.alert.modelHint')}</Typography>;
    };

    useEffect(() => {
        configService.selectConfig().then(config => {
            setProvideId(config?.job_alert_ai_provide_id ?? '');
        }).catch(() => {
            setProvideId('');
        });
    }, [configService]);

    return (
        <Box className="tm-set-card">
            <Box className="tm-set-card-head">
                <Box className="tm-set-card-head-main">
                    <Typography className="tm-set-card-title">{t('set.alert.title')}</Typography>
                    <Typography className="tm-set-card-desc">
                        {t('set.alert.desc')}
                    </Typography>
                </Box>
                <Box className="tm-set-card-head-action">
                    {renderError()}
                </Box>
            </Box>
            <Box className="tm-set-card-body">
                <ModelSelectRow label={t('set.alert.model')} hint={renderModelHint()} value={provideId} onChange={handleModelChange}/>
            </Box>
        </Box>
    );
}
