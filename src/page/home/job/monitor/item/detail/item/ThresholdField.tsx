import {useState} from 'react';
import {Box} from '@mui/material';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {EditField} from '@/page/common/components/input/EditField.tsx';
import {SelectField} from '@/page/common/components/input/SelectField.tsx';
import {splitScaled, toBaseText, unitScales} from '@/page/common/util/unitUtil.ts';

export interface ThresholdFieldProps {
    value: string;
    displayUnit: number;
    onChange: (value: string) => void;
}

export function ThresholdField(props: ThresholdFieldProps) {
    const {t} = useI18n();
    const {value, displayUnit, onChange} = props;
    const scales = unitScales(displayUnit);
    const initial = splitScaled(value, displayUnit);
    const [text, setText] = useState(initial.text);
    const [factor, setFactor] = useState(initial.factor);
    const options = scales.map(scale => ({value: String(scale.factor), label: scale.label}));

    const handleTextChange = (next: string): void => {
        setText(next);
        onChange(toBaseText(next, factor));
    };

    const handleFactorChange = (next: string): void => {
        const nextFactor = Number(next) || 1;
        setFactor(nextFactor);
        onChange(toBaseText(text, nextFactor));
    };

    if (scales.length === 0) {
        return <EditField label={t('home.common.threshold')} labelWidth={72} value={value} onChange={onChange} placeholder={t('home.job.thresholdPlaceholder')}/>;
    }

    return (
        <Box className="tm-job-field-row">
            <EditField label={t('home.common.threshold')} labelWidth={72} value={text} onChange={handleTextChange} placeholder={t('home.job.thresholdPlaceholder')}/>
            <Box className="tm-job-field-side">
                <SelectField value={String(factor)} onChange={handleFactorChange} options={options}/>
            </Box>
        </Box>
    );
}
