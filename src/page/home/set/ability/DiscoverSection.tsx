import type {ReactNode} from 'react';
import {Box, Typography} from '@mui/material';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {useEffect, useMemo, useState} from 'react';
import {isNotNil} from '@/common/util/StringUtil.ts';
import {useConfigService} from '@/common/context/useAppHooks.tsx';
import {ModelSelectRow} from '@/page/home/set/common/ModelSelectRow.tsx';
import {SelectField} from '@/page/common/components/input/SelectField.tsx';
import type {SelectOption} from '@/page/common/components/input/SelectField.tsx';
import {JOB_DISCOVER_PARALLELISM_DEFAULT, JOB_DISCOVER_PARALLELISM_MAX, JOB_DISCOVER_PARALLELISM_MIN} from '@/common/model/Config.ts';

export function DiscoverSection() {
    const {language, t, tf} = useI18n();
    const configService = useConfigService();

    const [provideId, setProvideId] = useState<string>('');
    const [parallelism, setParallelism] = useState<number>(JOB_DISCOVER_PARALLELISM_DEFAULT);
    const [error, setError] = useState<string | null>(null);

    const parallelismOptions = useMemo<SelectOption[]>(() => {
        return Array.from(
            {length: JOB_DISCOVER_PARALLELISM_MAX - JOB_DISCOVER_PARALLELISM_MIN + 1},
            (_, index) => {
                const value = String(JOB_DISCOVER_PARALLELISM_MIN + index);
                return {value, label: tf('set.discover.parallelUnit', {n: value})};
            },
        );
    }, [language, tf]);

    const persist = (nextProvideId: string, nextParallelism: number): void => {
        setError(null);
        configService.saveJobDiscover(nextProvideId === '' ? null : nextProvideId, nextParallelism).catch(() => {
            setError(t('common.saveFailed'));
        });
    };

    const handleModelChange = (next: string): void => {
        setProvideId(next);
        persist(next, parallelism);
    };

    const handleParallelismChange = (next: string): void => {
        const value = Number(next);
        setParallelism(value);
        persist(provideId, value);
    };

    const renderError = (): ReactNode => {
        if (!isNotNil(error)) {
            return null;
        }
        return <Typography className="tm-set-section-tip is-error">{error}</Typography>;
    };

    const renderModelHint = (): ReactNode => {
        if (provideId === '') {
            return <Typography className="tm-set-row-hint is-error">{t('set.discover.modelHintEmpty')}</Typography>;
        }
        return <Typography className="tm-set-row-hint">{t('set.discover.modelHint')}</Typography>;
    };

    useEffect(() => {
        configService.selectConfig().then(config => {
            setProvideId(config?.job_discover_ai_provide_id ?? '');
            setParallelism(config?.job_discover_parallelism || JOB_DISCOVER_PARALLELISM_DEFAULT);
        }).catch(() => {
            setProvideId('');
        });
    }, [configService]);

    return (
        <Box className="tm-set-card">
            <Box className="tm-set-card-head">
                <Box className="tm-set-card-head-main">
                    <Typography className="tm-set-card-title">{t('set.discover.title')}</Typography>
                    <Typography className="tm-set-card-desc">
                        {t('set.discover.desc')}
                    </Typography>
                </Box>
                <Box className="tm-set-card-head-action">
                    {renderError()}
                </Box>
            </Box>
            <Box className="tm-set-card-body">
                <ModelSelectRow label={t('set.discover.model')} hint={renderModelHint()} value={provideId} onChange={handleModelChange}/>
                <Box className="tm-set-row">
                    <Box className="tm-set-row-main">
                        <Typography className="tm-set-row-label">{t('set.discover.parallel')}</Typography>
                        <Typography className="tm-set-row-hint">{t('set.discover.parallelHint')}</Typography>
                    </Box>
                    <Box className="tm-set-row-control">
                        <SelectField value={String(parallelism)} onChange={handleParallelismChange} options={parallelismOptions}/>
                    </Box>
                </Box>
            </Box>
        </Box>
    );
}
