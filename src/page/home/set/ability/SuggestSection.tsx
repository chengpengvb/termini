import {useI18n} from '@/common/i18n/useI18n.ts';
import type {MouseEvent, ReactNode} from 'react';
import {useEffect, useMemo, useState} from 'react';
import {isNotNil} from '@/common/util/StringUtil.ts';
import {Box, Switch, Typography} from '@mui/material';
import {useConfigService} from '@/common/context/useAppHooks.tsx';
import {SelectField} from '@/page/common/components/input/SelectField.tsx';
import type {SelectOption} from '@/page/common/components/input/SelectField.tsx';
import {SUGGEST_COUNT_DEFAULT, SUGGEST_COUNT_MAX, SUGGEST_COUNT_MIN} from '@/common/model/Config.ts';

export function SuggestSection() {
    const {language, t, tf} = useI18n();
    const configService = useConfigService();

    const [enabled, setEnabled] = useState<boolean>(false);
    const [count, setCount] = useState<number>(SUGGEST_COUNT_DEFAULT);
    const [error, setError] = useState<string | null>(null);

    const countOptions = useMemo<SelectOption[]>(() => {
        return Array.from(
            {length: SUGGEST_COUNT_MAX - SUGGEST_COUNT_MIN + 1},
            (_, index) => {
                const value = String(SUGGEST_COUNT_MIN + index);
                return {value, label: tf('set.suggest.countUnit', {n: value})};
            },
        );
    }, [language, tf]);

    const persist = (nextEnabled: boolean, nextCount: number): void => {
        setError(null);
        configService.saveSuggest(nextEnabled, nextCount).catch(() => {
            setError(t('common.saveFailed'));
        });
    };

    const handleEnabledChange = (next: boolean): void => {
        setEnabled(next);
        persist(next, count);
    };

    const handleHeadClick = (): void => {
        handleEnabledChange(!enabled);
    };

    const handleSwitchClick = (event: MouseEvent): void => {
        event.stopPropagation();
    };

    const handleCountChange = (next: string): void => {
        const value = Number(next);
        setCount(value);
        persist(enabled, value);
    };

    const renderError = (): ReactNode => {
        if (!isNotNil(error)) {
            return null;
        }
        return <Typography className="tm-set-section-tip is-error">{error}</Typography>;
    };

    useEffect(() => {
        configService.selectConfig().then(config => {
            setEnabled(Boolean(config?.suggest_enabled));
            setCount(config?.suggest_count ?? SUGGEST_COUNT_DEFAULT);
        }).catch(() => {
            setEnabled(false);
        });
    }, [configService]);

    return (
        <Box className="tm-set-card">
            <Box className="tm-set-card-head is-toggle" onClick={handleHeadClick} role="button" tabIndex={0} onKeyDown={event => { if (event.key === 'Enter' || event.key === ' ') { event.preventDefault(); handleHeadClick(); } }}>
                <Box className="tm-set-card-head-main">
                    <Typography className="tm-set-card-title">{t('set.suggest.title')}</Typography>
                    <Typography className="tm-set-card-desc">
                        {t('set.suggest.desc')}
                    </Typography>
                </Box>
                <Box className="tm-set-card-head-action">
                    {renderError()}
                    <Switch size="small" className="tm-set-switch" checked={enabled} onClick={handleSwitchClick} onChange={e => handleEnabledChange(e.target.checked)}/>
                </Box>
            </Box>
            <Box className="tm-set-card-body">
                <Box className="tm-set-row">
                    <Box className="tm-set-row-main">
                        <Typography className="tm-set-row-label">{t('set.suggest.count')}</Typography>
                        <Typography className="tm-set-row-hint">{t('set.suggest.countHint')}</Typography>
                    </Box>
                    <Box className="tm-set-row-control">
                        <SelectField value={String(count)} onChange={handleCountChange} options={countOptions} sheetTitle={t('set.suggest.count')}/>
                    </Box>
                </Box>
            </Box>
        </Box>
    );
}
