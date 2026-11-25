import {Add, Remove} from '@mui/icons-material';
import {isNotNil} from '@/common/util/StringUtil.ts';
import {Box, IconButton, Typography} from '@mui/material';

export function StepperField(props: { label?: string; value: number; onChange: (value: number) => void; min?: number; max?: number; step?: number; suffix?: string; labelWidth?: number }) {
    const {label, value, onChange, min, max, step, suffix, labelWidth} = props;

    const resolveLabelWidth = (): number => {
        if (isNotNil(labelWidth)) {
            return labelWidth;
        }
        return 64;
    };

    const resolveStep = (): number => {
        return step ?? 1;
    };

    const clamp = (next: number): number => {
        let v = Math.round(next * 1000) / 1000;
        if (isNotNil(min) && v < min) {
            v = min;
        }
        if (isNotNil(max) && v > max) {
            v = max;
        }
        return v;
    };

    const resolveDisplayValue = (): string => {
        if (Number.isInteger(value)) {
            return String(value);
        }
        return String(Math.round(value * 1000) / 1000);
    };

    const handleDecrease = (): void => {
        onChange(clamp(value - resolveStep()));
    };

    const handleIncrease = (): void => {
        onChange(clamp(value + resolveStep()));
    };

    const isDecreaseDisabled = isNotNil(min) && value <= min;
    const isIncreaseDisabled = isNotNil(max) && value >= max;

    const hasLabel = isNotNil(label);
    const stepperClassName = hasLabel ? 'tm-stepper-field is-embedded' : 'tm-stepper-field';

    const stepper = (
        <Box className={stepperClassName} sx={{flexShrink: 0}}>
            <IconButton size="small" className="tm-stepper-field-btn" disabled={isDecreaseDisabled} onClick={handleDecrease}>
                <Remove fontSize="inherit"/>
            </IconButton>
            <Typography className="tm-stepper-field-value">{resolveDisplayValue()}{suffix ?? ''}</Typography>
            <IconButton size="small" className="tm-stepper-field-btn" disabled={isIncreaseDisabled} onClick={handleIncrease}>
                <Add fontSize="inherit"/>
            </IconButton>
        </Box>
    );

    if (!hasLabel) {
        return stepper;
    }

    return (
        <Box className="tm-edit-field" sx={{display: 'flex', alignItems: 'center', width: '100%', maxWidth: '100%', minWidth: 0, flexShrink: 0, boxSizing: 'border-box', minHeight: '38px', border: '1px solid var(--tm-border-subtle)', borderRadius: '7px', bgcolor: 'transparent', pl: 1, transition: 'border-color 120ms ease, background-color 120ms ease', '&:hover': {bgcolor: 'var(--tm-bg-hover)'}}}>
            <Typography className="tm-edit-field-label" sx={{fontSize: 13, color: 'var(--tm-text-tertiary)', width: resolveLabelWidth(), flexShrink: 0, whiteSpace: 'nowrap', userSelect: 'none'}}>{label}</Typography>
            <Box sx={{flex: 1, minWidth: 0, display: 'flex', justifyContent: 'flex-end'}}>{stepper}</Box>
        </Box>
    );
}
