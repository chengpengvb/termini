import type {ReactNode} from 'react';
import {Box, Typography} from '@mui/material';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {useEffect, useRef, useState} from 'react';
import {isNotNil} from '@/common/util/StringUtil.ts';
import {clampInt} from '@/page/common/util/numberInputUtil.ts';
import {useConfigService} from '@/common/context/useAppHooks.tsx';
import {EditField} from '@/page/common/components/input/EditField.tsx';
import {
    JOB_EXEC_TIMEOUT_SECS_DEFAULT, JOB_EXEC_TIMEOUT_SECS_MAX, JOB_EXEC_TIMEOUT_SECS_MIN,
    JOB_INDICATOR_VALUE_CACHE_SIZE_DEFAULT, JOB_INDICATOR_VALUE_CACHE_SIZE_MAX, JOB_INDICATOR_VALUE_CACHE_SIZE_MIN,
    JOB_WORKER_PARALLELISM_DEFAULT, JOB_WORKER_PARALLELISM_MAX, JOB_WORKER_PARALLELISM_MIN,
} from '@/common/model/Config.ts';

export function CollectSection() {
    const {t, tf} = useI18n();
    const configService = useConfigService();

    const savedRef = useRef<string>('');
    const [error, setError] = useState<string | null>(null);
    const [timeoutSecs, setTimeoutSecs] = useState<string>(String(JOB_EXEC_TIMEOUT_SECS_DEFAULT));
    const [parallelism, setParallelism] = useState<string>(String(JOB_WORKER_PARALLELISM_DEFAULT));
    const [cacheSize, setCacheSize] = useState<string>(String(JOB_INDICATOR_VALUE_CACHE_SIZE_DEFAULT));

    const persist = (nextTimeoutSecs: number, nextParallelism: number, nextCacheSize: number): void => {
        const key = `${nextTimeoutSecs}/${nextParallelism}/${nextCacheSize}`;
        if (key === savedRef.current) {
            return;
        }
        savedRef.current = key;
        setError(null);
        configService.saveJobCollect(nextTimeoutSecs, nextParallelism, nextCacheSize).catch(() => {
            setError(t('common.saveFailed'));
        });
    };

    const handleTimeoutBlur = (): void => {
        const value = clampInt(timeoutSecs, JOB_EXEC_TIMEOUT_SECS_MIN, JOB_EXEC_TIMEOUT_SECS_MAX, JOB_EXEC_TIMEOUT_SECS_DEFAULT);
        setTimeoutSecs(String(value));
        persist(value, Number(parallelism), Number(cacheSize));
    };

    const handleParallelismBlur = (): void => {
        const value = clampInt(parallelism, JOB_WORKER_PARALLELISM_MIN, JOB_WORKER_PARALLELISM_MAX, JOB_WORKER_PARALLELISM_DEFAULT);
        setParallelism(String(value));
        persist(Number(timeoutSecs), value, Number(cacheSize));
    };

    const handleCacheSizeBlur = (): void => {
        const value = clampInt(cacheSize, JOB_INDICATOR_VALUE_CACHE_SIZE_MIN, JOB_INDICATOR_VALUE_CACHE_SIZE_MAX, JOB_INDICATOR_VALUE_CACHE_SIZE_DEFAULT);
        setCacheSize(String(value));
        persist(Number(timeoutSecs), Number(parallelism), value);
    };

    const renderError = (): ReactNode => {
        if (!isNotNil(error)) {
            return null;
        }
        return <Typography className="tm-set-section-tip is-error">{error}</Typography>;
    };

    useEffect(() => {
        configService.selectConfig().then(config => {
            const nextTimeoutSecs = config?.job_exec_timeout_secs || JOB_EXEC_TIMEOUT_SECS_DEFAULT;
            const nextParallelism = config?.job_worker_parallelism || JOB_WORKER_PARALLELISM_DEFAULT;
            const nextCacheSize = config?.job_indicator_value_cache_size || JOB_INDICATOR_VALUE_CACHE_SIZE_DEFAULT;
            savedRef.current = `${nextTimeoutSecs}/${nextParallelism}/${nextCacheSize}`;
            setTimeoutSecs(String(nextTimeoutSecs));
            setParallelism(String(nextParallelism));
            setCacheSize(String(nextCacheSize));
        }).catch(() => {
            setError(t('common.loadFailed'));
        });
    }, [configService, t]);

    return (
        <Box className="tm-set-card">
            <Box className="tm-set-card-head">
                <Box className="tm-set-card-head-main">
                    <Typography className="tm-set-card-title">{t('set.collect.title')}</Typography>
                    <Typography className="tm-set-card-desc">
                        {t('set.collect.desc')}
                    </Typography>
                </Box>
                <Box className="tm-set-card-head-action">
                    {renderError()}
                </Box>
            </Box>
            <Box className="tm-set-card-body">
                <Box className="tm-set-row">
                    <Box className="tm-set-row-main">
                        <Typography className="tm-set-row-label">{t('set.collect.timeout')}</Typography>
                        <Typography className="tm-set-row-hint">{tf('set.collect.timeoutHint', {min: JOB_EXEC_TIMEOUT_SECS_MIN, max: JOB_EXEC_TIMEOUT_SECS_MAX})}</Typography>
                    </Box>
                    <Box className="tm-set-row-control">
                        <EditField value={timeoutSecs} onChange={setTimeoutSecs} onBlur={handleTimeoutBlur} placeholder={String(JOB_EXEC_TIMEOUT_SECS_DEFAULT)}/>
                    </Box>
                </Box>
                <Box className="tm-set-row">
                    <Box className="tm-set-row-main">
                        <Typography className="tm-set-row-label">{t('set.collect.parallel')}</Typography>
                        <Typography className="tm-set-row-hint">{tf('set.collect.parallelHint', {min: JOB_WORKER_PARALLELISM_MIN, max: JOB_WORKER_PARALLELISM_MAX})}</Typography>
                    </Box>
                    <Box className="tm-set-row-control">
                        <EditField value={parallelism} onChange={setParallelism} onBlur={handleParallelismBlur} placeholder={String(JOB_WORKER_PARALLELISM_DEFAULT)}/>
                    </Box>
                </Box>
                <Box className="tm-set-row">
                    <Box className="tm-set-row-main">
                        <Typography className="tm-set-row-label">{t('set.collect.cache')}</Typography>
                        <Typography className="tm-set-row-hint">{tf('set.collect.cacheHint', {min: JOB_INDICATOR_VALUE_CACHE_SIZE_MIN, max: JOB_INDICATOR_VALUE_CACHE_SIZE_MAX})}</Typography>
                    </Box>
                    <Box className="tm-set-row-control">
                        <EditField value={cacheSize} onChange={setCacheSize} onBlur={handleCacheSizeBlur} placeholder={String(JOB_INDICATOR_VALUE_CACHE_SIZE_DEFAULT)}/>
                    </Box>
                </Box>
            </Box>
        </Box>
    );
}
