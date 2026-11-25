import {useState} from 'react';
import type {KeyboardEvent} from 'react';
import {Box, Typography} from '@mui/material';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {isNil, isNotNil} from '@/common/util/StringUtil.ts';
import {eventToKeyBind, formatKeyBind, type KeyBind} from '@/common/util/KeyBindUtil.ts';

export function KeyBindField(props: { label?: string; labelWidth?: number; value: KeyBind | null; defaultValue: KeyBind; allowModifiers?: boolean; onChange: (bind: KeyBind | null) => void }) {
    const {t} = useI18n();
    const [listening, setListening] = useState<boolean>(false);
    const {label, labelWidth, value, defaultValue, allowModifiers, onChange} = props;

    const resolveLabelWidth = (): number => {
        if (isNotNil(labelWidth)) {
            return labelWidth;
        }
        return 64;
    };

    const resolveDisplayBind = (): KeyBind => {
        if (isNil(value)) {
            return defaultValue;
        }
        return value;
    };

    const resolveDisplayText = (): string => {
        if (listening) {
            return t('common.pressKey');
        }
        return formatKeyBind(resolveDisplayBind());
    };

    const isModifierKey = (key: string): boolean => {
        return key === 'Control' || key === 'Alt' || key === 'Shift' || key === 'Meta';
    };

    const handleClick = (): void => {
        setListening(!listening);
    };

    const handleBlur = (): void => {
        setListening(false);
    };

    const handleKeyDown = (event: KeyboardEvent<HTMLDivElement>): void => {
        event.preventDefault();
        event.stopPropagation();
        if (!listening) {
            if (event.key === 'Backspace' || event.key === 'Delete') {
                onChange(null);
            }
            return;
        }
        if (event.key === 'Escape') {
            setListening(false);
            return;
        }
        if (isModifierKey(event.key)) {
            return;
        }
        const bind = eventToKeyBind(event);
        if (allowModifiers !== true && (bind.ctrl || bind.alt || bind.meta)) {
            return;
        }
        onChange(bind);
        setListening(false);
    };

    return (
        <Box className="tm-edit-field" tabIndex={0} onClick={handleClick} onBlur={handleBlur} onKeyDown={handleKeyDown} sx={{display: 'flex', alignItems: 'center', width: '100%', maxWidth: '100%', minWidth: 0, flexShrink: 0, boxSizing: 'border-box', minHeight: '38px', cursor: 'pointer', border: '1px solid var(--tm-border-subtle)', borderRadius: '7px', bgcolor: 'transparent', outline: 'none', transition: 'border-color 120ms ease, background-color 120ms ease', '&:hover': {bgcolor: 'var(--tm-bg-hover)'}, '&:focus-within': {bgcolor: 'var(--tm-bg-hover)', borderColor: 'var(--tm-border-accent)'}}}>
            {isNotNil(label) ? (
                <Typography className="tm-edit-field-label" sx={{fontSize: 13, color: 'var(--tm-text-tertiary)', pl: 1, width: resolveLabelWidth(), flexShrink: 0, whiteSpace: 'nowrap', userSelect: 'none'}}>{label}</Typography>
            ) : null}
            <Box className="tm-edit-field-input" sx={{flex: 1, minWidth: 0, px: 1, py: 0.5, fontSize: 13, color: listening ? 'var(--tm-text-tertiary)' : 'var(--tm-text-primary)', userSelect: 'none'}}>
                {resolveDisplayText()}
            </Box>
        </Box>
    );
}
