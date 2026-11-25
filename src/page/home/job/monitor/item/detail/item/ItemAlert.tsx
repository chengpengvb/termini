import {useMemo} from 'react';
import {Box, Typography} from '@mui/material';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {ThresholdField} from './ThresholdField.tsx';
import {EditField} from '@/page/common/components/input/EditField.tsx';
import {SelectField} from '@/page/common/components/input/SelectField.tsx';
import {TextareaField} from '@/page/common/components/input/TextareaField.tsx';
import {parseNonNegativeInt, resolveDisplayNumber} from '@/page/common/util/numberInputUtil.ts';
import {JOB_ALERT_AI_ON, JOB_ALERT_RULE_OP, type JobIndicator} from '@/common/model/JobIndicator.ts';

export interface ItemAlertProps {
    draft: JobIndicator;
    onPatch: (next: Partial<JobIndicator>) => void;
}

export function ItemAlert(props: ItemAlertProps) {
    const {draft, onPatch} = props;
    const {t} = useI18n();
    const repairConfirm = draft.alert_ai_on === JOB_ALERT_AI_ON.REPAIR_CONFIRM;

    const alertEnabledOptions = useMemo(() => [
        {value: '0', label: t('home.job.alertOff')},
        {value: '1', label: t('home.job.alertOn')},
    ], [t]);

    const ruleOptions = useMemo(() => [
        {value: String(JOB_ALERT_RULE_OP.GT), label: t('home.job.ruleGt')},
        {value: String(JOB_ALERT_RULE_OP.GTE), label: t('home.job.ruleGte')},
        {value: String(JOB_ALERT_RULE_OP.LT), label: t('home.job.ruleLt')},
        {value: String(JOB_ALERT_RULE_OP.LTE), label: t('home.job.ruleLte')},
        {value: String(JOB_ALERT_RULE_OP.EQ), label: t('home.job.ruleEq')},
    ], [t]);

    const aiOptions = useMemo(() => [
        {value: String(JOB_ALERT_AI_ON.OFF), label: t('home.job.aiOff')},
        {value: String(JOB_ALERT_AI_ON.ANALYZE), label: t('home.job.aiAnalyze')},
        {value: String(JOB_ALERT_AI_ON.REPAIR_CONFIRM), label: t('home.job.aiRepair')},
    ], [t]);

    const handleAiOnChange = (raw: string): void => {
        const alert_ai_on = Number(raw);
        if (alert_ai_on === JOB_ALERT_AI_ON.REPAIR_CONFIRM) {
            onPatch({alert_ai_on});
            return;
        }
        onPatch({alert_ai_on, alert_ai_repair_message: null});
    };

    const handleRepairMessageChange = (value: string): void => {
        const trimmed = value.trim();
        onPatch({alert_ai_repair_message: trimmed.length === 0 ? null : value});
    };

    return (
        <Box className="tm-job-detail-section">
            <Typography className="tm-job-section-title">{t('home.job.alertSection')}</Typography>
            <SelectField label={t('home.job.alertEnabled')} labelWidth={72} value={draft.alert_enabled ? '1' : '0'} onChange={v => onPatch({alert_enabled: v === '1'})} options={alertEnabledOptions}/>
            {draft.alert_enabled ? (
                <>
                    <SelectField label={t('home.job.rule')} labelWidth={72} value={String(draft.alert_rule_op)} onChange={v => onPatch({alert_rule_op: Number(v)})} options={ruleOptions}/>
                    <ThresholdField key={`threshold:${draft.display_unit}`} value={draft.alert_threshold} displayUnit={draft.display_unit} onChange={v => onPatch({alert_threshold: v})}/>
                    <EditField label={t('home.job.sensitivity')} labelWidth={72} value={resolveDisplayNumber(draft.alert_sensitivity)} onChange={v => onPatch({alert_sensitivity: parseNonNegativeInt(v)})} placeholder={t('home.job.sensitivityPlaceholder')}/>
                    <SelectField label={t('home.job.aiUpgrade')} labelWidth={72} value={String(draft.alert_ai_on)} onChange={handleAiOnChange} options={aiOptions}/>
                    {repairConfirm ? (
                        <TextareaField label={t('home.job.repairMessage')} labelWidth={72} value={draft.alert_ai_repair_message ?? ''} onChange={handleRepairMessageChange} placeholder={t('home.job.repairPlaceholder')} expandable minRows={1} maxRows={1}/>
                    ) : null}
                </>
            ) : (
                <Typography className="tm-job-drawer-hint">{t('home.job.alertOffHint')}</Typography>
            )}
        </Box>
    );
}
