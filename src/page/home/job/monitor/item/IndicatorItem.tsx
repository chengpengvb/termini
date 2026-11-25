import {ChevronRight} from '@mui/icons-material';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {isNil} from '@/common/util/StringUtil.ts';
import {useEffect, useRef, useState} from 'react';
import {Box, ButtonBase, Typography} from '@mui/material';
import {Sparkline} from '../../common/chart/Sparkline.tsx';
import {JOB_ALERT_AI_ON} from '@/common/model/JobIndicator.ts';
import type {JobIndicator} from '@/common/model/JobIndicator.ts';
import type {JobDevice2Indicator} from '@/common/model/JobDevice.ts';
import type {RespMetricItem} from '@/common/model/resp/RespMetricItem.ts';
import type {JobIndicatorValue} from '@/common/model/JobIndicatorValue.ts';
import {displayValue, formatInterval, resolveMetricStatus, toneOf, toSamples, type JobStatus} from '@/page/common/util/jobUtil.ts';

export interface IndicatorItemProps {
    selected?: boolean;
    link: JobDevice2Indicator;
    metric?: RespMetricItem | null;
    onSelect?: (link: JobDevice2Indicator) => void;
}

export function IndicatorItem(props: IndicatorItemProps) {
    const {t, tf} = useI18n();
    const settledRef = useRef(false);
    const {link, metric, selected, onSelect} = props;
    const indicator = link.job_indicator;
    const [flash, setFlash] = useState(false);
    const updatedAt = metric?.updated_at ?? null;
    const metricStatus = resolveMetricStatus(metric);
    const updatedAtRef = useRef<number | null>(null);
    const name = indicator?.name || link.job_indicator_id;
    const aiOn = indicator != null && indicator.alert_ai_on !== JOB_ALERT_AI_ON.OFF;
    const last = metric?.values?.length ? metric.values[metric.values.length - 1] : null;

    const resolveStatusLabel = (status: JobStatus): string => t(`home.job.status.${status}`);

    const formatAgoLabel = (millis: number | null | undefined): string => {
        if (isNil(millis) || millis <= 0) {
            return t('home.job.ago.notCollected');
        }
        const diff = Math.max(0, Date.now() - millis);
        if (diff < 60_000) {
            return t('home.job.ago.justNow');
        }
        if (diff < 3600_000) {
            return tf('home.job.ago.minutes', {n: Math.floor(diff / 60_000)});
        }
        if (diff < 24 * 3600_000) {
            return tf('home.job.ago.hours', {n: Math.floor(diff / 3600_000)});
        }
        if (diff < 30 * 24 * 3600_000) {
            return tf('home.job.ago.days', {n: Math.floor(diff / (24 * 3600_000))});
        }
        if (diff < 365 * 24 * 3600_000) {
            return tf('home.job.ago.months', {n: Math.floor(diff / (30 * 24 * 3600_000))});
        }
        return tf('home.job.ago.years', {n: Math.floor(diff / (365 * 24 * 3600_000))});
    };

    const resolveDisplayValue = (item: JobIndicator | null | undefined, value: JobIndicatorValue | null | undefined): string => {
        if (value != null && !value.success) {
            return t('home.job.displayCollectFailed');
        }
        return displayValue(item, value);
    };

    const handleFlashEnd = (): void => {
        setFlash(false);
    };

    const handleClick = (): void => {
        onSelect?.(link);
    };

    const resolveClassName = (): string => {
        let next = `tm-device-item tm-job-metric-row is-${metricStatus}`;
        if (selected) {
            next += ' is-selected';
        }
        if (flash) {
            next += ' is-flash';
        }
        return next;
    };

    useEffect(() => {
        if (!settledRef.current) {
            settledRef.current = true;
            updatedAtRef.current = updatedAt;
            return;
        }
        if (updatedAt === updatedAtRef.current) {
            return;
        }
        updatedAtRef.current = updatedAt;
        setFlash(true);
    }, [updatedAt]);

    return (
        <ButtonBase disableRipple className={resolveClassName()} onClick={handleClick} onAnimationEnd={handleFlashEnd}>
            <Box className={`tm-job-status-dot is-${metricStatus}`} title={resolveStatusLabel(metricStatus)}/>
            <Box className="tm-job-metric-main">
                <Typography className="tm-job-metric-name">{name}</Typography>
                <Typography className="tm-job-metric-sub">
                    {t('home.common.period')} {formatInterval(link.interval_secs)} · {formatAgoLabel(updatedAt)}
                    {aiOn ? t('home.job.aiUpgradeOn') : ''}
                </Typography>
            </Box>
            <Typography className={`tm-job-metric-value is-${toneOf(metricStatus)}`}>
                {resolveDisplayValue(indicator, last)}
            </Typography>
            <Sparkline samples={toSamples(metric?.values ?? [])} tone={toneOf(metricStatus)}/>
            <ChevronRight className="tm-job-metric-chevron"/>
        </ButtonBase>
    );
}
