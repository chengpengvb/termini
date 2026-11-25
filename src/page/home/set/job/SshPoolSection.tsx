import type {ReactNode} from 'react';
import {Box, Typography} from '@mui/material';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {useEffect, useRef, useState} from 'react';
import {isNotNil} from '@/common/util/StringUtil.ts';
import {clampInt} from '@/page/common/util/numberInputUtil.ts';
import {useConfigService} from '@/common/context/useAppHooks.tsx';
import {EditField} from '@/page/common/components/input/EditField.tsx';
import {JOB_SSH_POOL_IDLE_SECS_DEFAULT, JOB_SSH_POOL_IDLE_SECS_MAX, JOB_SSH_POOL_IDLE_SECS_MIN, JOB_SSH_POOL_MAX_SIZE_DEFAULT, JOB_SSH_POOL_MAX_SIZE_MAX, JOB_SSH_POOL_MAX_SIZE_MIN} from '@/common/model/Config.ts';

export function SshPoolSection() {
    const {t, tf} = useI18n();
    const configService = useConfigService();

    const savedRef = useRef<string>('');
    const [error, setError] = useState<string | null>(null);
    const [maxSize, setMaxSize] = useState<string>(String(JOB_SSH_POOL_MAX_SIZE_DEFAULT));
    const [idleSecs, setIdleSecs] = useState<string>(String(JOB_SSH_POOL_IDLE_SECS_DEFAULT));

    const persist = (nextMaxSize: number, nextIdleSecs: number): void => {
        const key = `${nextMaxSize}/${nextIdleSecs}`;
        if (key === savedRef.current) {
            return;
        }
        savedRef.current = key;
        setError(null);
        configService.saveJobSshPool(nextMaxSize, nextIdleSecs).catch(() => {
            setError(t('common.saveFailed'));
        });
    };

    const handleMaxSizeBlur = (): void => {
        const value = clampInt(maxSize, JOB_SSH_POOL_MAX_SIZE_MIN, JOB_SSH_POOL_MAX_SIZE_MAX, JOB_SSH_POOL_MAX_SIZE_DEFAULT);
        setMaxSize(String(value));
        persist(value, Number(idleSecs));
    };

    const handleIdleSecsBlur = (): void => {
        const value = clampInt(idleSecs, JOB_SSH_POOL_IDLE_SECS_MIN, JOB_SSH_POOL_IDLE_SECS_MAX, JOB_SSH_POOL_IDLE_SECS_DEFAULT);
        setIdleSecs(String(value));
        persist(Number(maxSize), value);
    };

    const renderError = (): ReactNode => {
        if (!isNotNil(error)) {
            return null;
        }
        return <Typography className="tm-set-section-tip is-error">{error}</Typography>;
    };

    useEffect(() => {
        configService.selectConfig().then(config => {
            const nextMaxSize = config?.job_ssh_pool_max_size || JOB_SSH_POOL_MAX_SIZE_DEFAULT;
            const nextIdleSecs = config?.job_ssh_pool_idle_secs || JOB_SSH_POOL_IDLE_SECS_DEFAULT;
            savedRef.current = `${nextMaxSize}/${nextIdleSecs}`;
            setMaxSize(String(nextMaxSize));
            setIdleSecs(String(nextIdleSecs));
        }).catch(() => {
            setError(t('common.loadFailed'));
        });
    }, [configService, t]);

    return (
        <Box className="tm-set-card">
            <Box className="tm-set-card-head">
                <Box className="tm-set-card-head-main">
                    <Typography className="tm-set-card-title">{t('set.sshPool.title')}</Typography>
                    <Typography className="tm-set-card-desc">
                        {t('set.sshPool.desc')}
                    </Typography>
                </Box>
                <Box className="tm-set-card-head-action">
                    {renderError()}
                </Box>
            </Box>
            <Box className="tm-set-card-body">
                <Box className="tm-set-row">
                    <Box className="tm-set-row-main">
                        <Typography className="tm-set-row-label">{t('set.sshPool.size')}</Typography>
                        <Typography className="tm-set-row-hint">{tf('set.sshPool.sizeHint', {min: JOB_SSH_POOL_MAX_SIZE_MIN, max: JOB_SSH_POOL_MAX_SIZE_MAX})}</Typography>
                    </Box>
                    <Box className="tm-set-row-control">
                        <EditField value={maxSize} onChange={setMaxSize} onBlur={handleMaxSizeBlur} placeholder={String(JOB_SSH_POOL_MAX_SIZE_DEFAULT)}/>
                    </Box>
                </Box>
                <Box className="tm-set-row">
                    <Box className="tm-set-row-main">
                        <Typography className="tm-set-row-label">{t('set.sshPool.idle')}</Typography>
                        <Typography className="tm-set-row-hint">{tf('set.sshPool.idleHint', {min: JOB_SSH_POOL_IDLE_SECS_MIN, max: JOB_SSH_POOL_IDLE_SECS_MAX})}</Typography>
                    </Box>
                    <Box className="tm-set-row-control">
                        <EditField value={idleSecs} onChange={setIdleSecs} onBlur={handleIdleSecsBlur} placeholder={String(JOB_SSH_POOL_IDLE_SECS_DEFAULT)}/>
                    </Box>
                </Box>
            </Box>
        </Box>
    );
}
