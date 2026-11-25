import {useI18n} from '@/common/i18n/useI18n.ts';
import {isNil} from '@/common/util/StringUtil.ts';
import {Box, ButtonBase, Typography} from '@mui/material';
import type {JobIndicator} from '@/common/model/JobIndicator.ts';
import {displayAlertTriggerValue, formatTime} from '@/page/common/util/jobUtil.ts';
import {CheckBox, CheckBoxOutlineBlank, IndeterminateCheckBox} from '@mui/icons-material';
import {JOB_ALERT_STATUS, type JobIndicatorAlert} from '@/common/model/JobIndicatorAlert.ts';

export interface AlertListProps {
    checkedIds: string[];
    onToggleAll: () => void;
    selectedId: string | null;
    alerts: JobIndicatorAlert[];
    onToggle: (id: string) => void;
    indicator?: JobIndicator | null;
    onSelect: (alert: JobIndicatorAlert) => void;
}

export function AlertList(props: AlertListProps) {
    const {t, tf} = useI18n();
    const {alerts, selectedId, checkedIds, indicator, onToggle, onToggleAll, onSelect} = props;
    const allSelected = alerts.length > 0 && checkedIds.length === alerts.length;
    const someSelected = checkedIds.length > 0 && !allSelected;

    const resolveFireTypeLabel = (fireType: number | null | undefined): string => {
        if (fireType == null) {
            return t('home.common.emDash');
        }
        return t(`home.job.alert.fireType.${fireType}`);
    };

    const formatAgoLabel = (millis: number | null | undefined): string => {
        if (isNil(millis) || millis <= 0) {
            return t('home.common.emDash');
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

    const renderHeadCheck = () => {
        if (allSelected) {
            return <CheckBox className="tm-job-table-check"/>;
        }
        if (someSelected) {
            return <IndeterminateCheckBox className="tm-job-table-check"/>;
        }
        return <CheckBoxOutlineBlank className="tm-job-table-check"/>;
    };

    if (alerts.length === 0) {
        return (
            <Box className="tm-job-alarm-empty">
                <Typography className="tm-job-muted">{t('home.job.alert.empty')}</Typography>
            </Box>
        );
    }

    return (
        <Box className="tm-job-alarm-list">
            <ButtonBase disableRipple className="tm-job-alarm-list-bar" onClick={onToggleAll} aria-label={allSelected ? t('home.common.deselectAll') : t('home.common.selectAll')}>
                <Box className="tm-job-table-check-slot">{renderHeadCheck()}</Box>
                <Typography className="tm-job-alarm-list-bar-label">{allSelected ? t('home.common.deselectAll') : t('home.common.selectAll')}</Typography>
            </ButtonBase>
            {alerts.map(alert => {
                const firing = alert.status === JOB_ALERT_STATUS.FIRING;
                const tone = firing ? 'firing' : 'ok';
                const selected = selectedId === alert.id;
                const checked = checkedIds.includes(alert.id);
                const rowClass = [
                    'tm-job-alarm-row',
                    `is-${tone}`,
                    selected ? 'is-selected' : null,
                    checked ? 'is-checked' : null,
                ].filter(Boolean).join(' ');
                const triggerValue = displayAlertTriggerValue(indicator ?? alert.job_indicator, alert);
                return (
                    <ButtonBase key={alert.id} component="div" disableRipple className={rowClass} onClick={() => onSelect(alert)}>
                        <ButtonBase disableRipple className="tm-job-table-check-btn" onClick={event => { event.stopPropagation(); onToggle(alert.id); }} aria-label={checked ? t('home.common.deselect') : t('home.common.select')}>
                            {checked ? <CheckBox className="tm-job-table-check"/> : <CheckBoxOutlineBlank className="tm-job-table-check"/>}
                        </ButtonBase>
                        <Box className="tm-job-alarm-row-main">
                            <Box className="tm-job-alarm-row-title">
                                <Box className={`tm-job-status-dot is-${tone}`}/>
                                <Typography component="span" className="tm-job-alarm-row-status">{firing ? t('home.job.alert.firing') : t('home.job.alert.normal')}</Typography>
                                <Typography component="span" className="tm-job-alarm-row-type">{resolveFireTypeLabel(alert.fire_type)}</Typography>
                                <Typography component="span" className="tm-job-alarm-row-time" title={formatTime(alert.alert_at)}>{formatAgoLabel(alert.alert_at)}</Typography>
                            </Box>
                            <Typography className="tm-job-alarm-row-meta">
                                <Box component="span" className="tm-job-alarm-row-meta-label">{t('home.job.alert.triggerValue')}</Box>
                                <Box component="span" className="tm-job-alarm-row-meta-value">{triggerValue}</Box>
                            </Typography>
                        </Box>
                    </ButtonBase>
                );
            })}
        </Box>
    );
}
