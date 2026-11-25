import {Box, Typography} from '@mui/material';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {isEmpty, isNil} from '@/common/util/StringUtil.ts';
import {AgentMarkdown} from '@/page/agent/common/AgentMarkdown.tsx';
import {protocolLabel, ruleOpLabel} from '@/common/model/JobIndicator.ts';
import {JOB_ALERT_STATUS, type JobIndicatorAlert} from '@/common/model/JobIndicatorAlert.ts';
import {displayAlertResolvedValue, displayAlertTriggerValue, displayThreshold, formatTime, indicatorCommand} from '@/page/common/util/jobUtil.ts';

export interface AlertDetailProps {
    alert: JobIndicatorAlert;
}

function Field(props: {label: string; value: string}) {
    const {label, value} = props;
    return (
        <Box className="tm-job-alarm-field">
            <Typography className="tm-job-alarm-field-label">{label}</Typography>
            <Typography className="tm-job-alarm-field-value">{value}</Typography>
        </Box>
    );
}

export function AlertDetail(props: AlertDetailProps) {
    const {alert} = props;
    const {t, tf} = useI18n();
    const indicator = alert.job_indicator;
    const firing = alert.status === JOB_ALERT_STATUS.FIRING;
    const tone = firing ? 'firing' : 'ok';
    const command = indicatorCommand(indicator);

    const resolveFireTypeLabel = (fireType: number | null | undefined): string => {
        if (fireType == null) {
            return t('home.common.emDash');
        }
        return t(`home.job.alert.fireType.${fireType}`);
    };

    const resolveResolveTypeLabel = (resolvedType: number | null | undefined): string => {
        if (resolvedType == null) {
            return t('home.common.emDash');
        }
        return t(`home.job.alert.resolveType.${resolvedType}`);
    };

    const resolveParserTypeLabel = (parserType: number | null | undefined): string => {
        if (parserType == null) {
            return t('home.common.emDash');
        }
        return t(`home.job.alert.parserType.${parserType}`);
    };

    const emDash = t('home.common.emDash');
    const parseValue = isNil(indicator)
        ? emDash
        : `${resolveParserTypeLabel(indicator.parser_type)} · ${isEmpty(indicator.parse_expr) ? emDash : indicator.parse_expr}`;

    const bannerMeta = [
        tf('home.job.alert.triggerAtTime', {at: formatTime(alert.alert_at)}),
        isNil(alert.resolved_at) ? '' : tf('home.job.alert.resolvedAtTime', {at: formatTime(alert.resolved_at)}),
        firing ? '' : ` · ${resolveResolveTypeLabel(alert.resolved_type)}`,
        firing ? '' : tf('home.job.alert.bannerResolvedValue', {value: displayAlertResolvedValue(indicator, alert)}),
        isNil(indicator) ? '' : tf('home.job.alert.bannerSensitivity', {n: indicator.alert_sensitivity}),
    ].join('');

    return (
        <Box className="tm-job-alarm-detail">
            <Box className={`tm-job-alarm-banner is-${tone}`}>
                <Typography className="tm-job-alarm-banner-title">
                    {tf('home.job.alert.bannerTitle', {
                        fireType: resolveFireTypeLabel(alert.fire_type),
                        trigger: displayAlertTriggerValue(indicator, alert),
                        op: ruleOpLabel(alert.alert_rule_op),
                        threshold: displayThreshold(indicator, alert.alert_threshold),
                    })}
                </Typography>
                <Typography className="tm-job-muted">{bannerMeta}</Typography>
            </Box>

            <Box className="tm-job-detail-section">
                <Typography className="tm-job-section-title">{t('home.job.alert.snapshot')}</Typography>
                <Field label={t('home.job.protocol')} value={isNil(indicator) ? emDash : protocolLabel(indicator.protocol)}/>
                <Field label={t('home.job.alert.command')} value={isEmpty(command) ? emDash : command}/>
                <Field label={t('home.job.alert.parse')} value={parseValue}/>
                <Field label={t('home.job.alert.triggerType')} value={resolveFireTypeLabel(alert.fire_type)}/>
                <Field label={t('home.job.alert.triggerValue')} value={displayAlertTriggerValue(indicator, alert)}/>
                <Field label={t('home.job.alert.resolvedValue')} value={firing ? emDash : displayAlertResolvedValue(indicator, alert)}/>
                <Field label={t('home.job.alert.resolvedType')} value={firing ? emDash : resolveResolveTypeLabel(alert.resolved_type)}/>
                {alert.job_indicator_value?.error ? <Field label={t('home.job.alert.collectError')} value={alert.job_indicator_value.error}/> : null}
            </Box>

            <Box className="tm-job-detail-section">
                <Typography className="tm-job-section-title">{t('home.job.alert.aiSummary')}</Typography>
                {isEmpty(alert.ai_summary) ? (
                    <Typography className="tm-job-drawer-hint">{t('home.job.alert.aiEmpty')}</Typography>
                ) : (
                    <AgentMarkdown content={alert.ai_summary ?? ''} className="tm-agent-md tm-job-alarm-ai-summary"/>
                )}
            </Box>
        </Box>
    );
}
