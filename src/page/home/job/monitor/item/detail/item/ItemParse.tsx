import {Box, Typography} from '@mui/material';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {useEffect, useMemo, useState} from 'react';
import {DEFAULT_REGEX_EXPR} from '@/page/common/util/jobUtil.ts';
import {EditField} from '@/page/common/components/input/EditField.tsx';
import {SelectField} from '@/page/common/components/input/SelectField.tsx';
import {TextareaField} from '@/page/common/components/input/TextareaField.tsx';
import {JOB_DISPLAY_UNIT, JOB_PARSER_TYPE, type JobIndicator} from '@/common/model/JobIndicator.ts';

export interface ItemParseProps {
    draft: JobIndicator;
    onPatch: (next: Partial<JobIndicator>) => void;
}

function defaultParseExpr(parserType: number): string {
    if (parserType === JOB_PARSER_TYPE.JSON_PATH) {
        return '$.value';
    }
    if (parserType === JOB_PARSER_TYPE.EXIT_CODE) {
        return '';
    }
    return DEFAULT_REGEX_EXPR;
}

function rangeText(value: number | null): string {
    if (value == null) {
        return '';
    }
    return String(value);
}

export function ItemParse(props: ItemParseProps) {
    const {draft, onPatch} = props;
    const {t} = useI18n();
    const needsExpr = draft.parser_type !== JOB_PARSER_TYPE.EXIT_CODE;
    const [rangeMinText, setRangeMinText] = useState<string>(() => rangeText(draft.range_min));
    const [rangeMaxText, setRangeMaxText] = useState<string>(() => rangeText(draft.range_max));

    const parseOptions = useMemo(() => [
        {value: String(JOB_PARSER_TYPE.REGEX), label: t('home.job.parseRegex')},
        {value: String(JOB_PARSER_TYPE.JSON_PATH), label: t('home.job.parseJsonPath')},
        {value: String(JOB_PARSER_TYPE.EXIT_CODE), label: t('home.job.parseExitCode')},
    ], [t]);

    const displayUnitOptions = useMemo(() => [
        {value: String(JOB_DISPLAY_UNIT.NONE), label: t('home.job.displayNone')},
        {value: String(JOB_DISPLAY_UNIT.BYTES), label: t('home.job.displayBytes')},
        {value: String(JOB_DISPLAY_UNIT.BITRATE), label: t('home.job.displayBitrate')},
        {value: String(JOB_DISPLAY_UNIT.MILLIS), label: t('home.job.displayMillis')},
        {value: String(JOB_DISPLAY_UNIT.SECONDS), label: t('home.job.displaySeconds')},
        {value: String(JOB_DISPLAY_UNIT.COUNT), label: t('home.job.displayCount')},
    ], [t]);

    const parseExprPlaceholder = (parserType: number): string => {
        if (parserType === JOB_PARSER_TYPE.JSON_PATH) {
            return t('home.job.parseJsonExample');
        }
        if (parserType === JOB_PARSER_TYPE.EXIT_CODE) {
            return t('home.job.parseExitCodeHint');
        }
        return t('home.job.parseRegexExample');
    };

    const handleParserChange = (raw: string): void => {
        const parserType = Number(raw);
        onPatch({parser_type: parserType, parse_expr: defaultParseExpr(parserType)});
    };

    const handleRangeMinChange = (raw: string): void => {
        setRangeMinText(raw);
        const trimmed = raw.trim();
        if (trimmed === '') {
            onPatch({range_min: null});
            return;
        }
        const value = Number(trimmed);
        if (!Number.isFinite(value)) {
            return;
        }
        onPatch({range_min: value});
    };

    const handleRangeMaxChange = (raw: string): void => {
        setRangeMaxText(raw);
        const trimmed = raw.trim();
        if (trimmed === '') {
            onPatch({range_max: null});
            return;
        }
        const value = Number(trimmed);
        if (!Number.isFinite(value)) {
            return;
        }
        onPatch({range_max: value});
    };

    useEffect(() => {
        setRangeMinText(rangeText(draft.range_min));
        setRangeMaxText(rangeText(draft.range_max));
    }, [draft.id]);

    return (
        <Box className="tm-job-detail-section">
            <Typography className="tm-job-section-title">{t('home.job.parseSection')}</Typography>
            <SelectField label={t('home.job.parseMethod')} labelWidth={72} value={String(draft.parser_type)} onChange={handleParserChange} options={parseOptions}/>
            {needsExpr ? (
                <EditField label={t('home.job.parseExpr')} labelWidth={72} value={draft.parse_expr} onChange={v => onPatch({parse_expr: v})} placeholder={parseExprPlaceholder(draft.parser_type)}/>
            ) : (
                <Typography className="tm-job-drawer-hint">{t('home.job.parseExitCodeDesc')}</Typography>
            )}
            <TextareaField label={t('home.job.transform')} labelWidth={72} value={draft.transform_expr} onChange={v => onPatch({transform_expr: v})} placeholder={t('home.job.transformPlaceholder')} expandable language="javascript" minRows={1} maxRows={1}/>
            <SelectField label={t('home.job.displayUnit')} labelWidth={72} value={String(draft.display_unit)} onChange={v => onPatch({display_unit: Number(v)})} options={displayUnitOptions}/>
            <EditField label={t('home.job.rangeMin')} labelWidth={72} value={rangeMinText} onChange={handleRangeMinChange} placeholder={t('home.job.rangeMinPlaceholder')}/>
            <EditField label={t('home.job.rangeMax')} labelWidth={72} value={rangeMaxText} onChange={handleRangeMaxChange} placeholder={t('home.job.rangeMaxPlaceholder')}/>
        </Box>
    );
}
