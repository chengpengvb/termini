import { useState } from 'react';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { Box, ButtonBase, Typography } from '@mui/material';
import { AgentMarkdown } from '@/page/agent/common/AgentMarkdown.tsx';
import { isEmpty, isNotEmpty, isNotNil } from '@/common/util/StringUtil.ts';
import { TextAreaField } from '@/page/home/keys/edit/section/shared/TextAreaField.tsx';

type MarkdownFieldMode = 'preview' | 'edit';

interface MarkdownFieldProps {
    rows?: number;
    label?: string;
    className?: string;
    placeholder?: string;
    value: string | null | undefined;
    onChange: (value: string) => void;
}

export function MarkdownField(props: MarkdownFieldProps) {
    const {t} = useI18n();
    const { label, value, onChange, placeholder, rows, className } = props;
    const [mode, setMode] = useState<MarkdownFieldMode>(() => {
        if (isNotEmpty(value)) {
            return 'preview';
        }
        return 'edit';
    });

    function resolveContent(): string {
        if (!value) {
            return '';
        }
        return value;
    }

    function resolveTextAreaValue(): string | undefined {
        if (isNotNil(value)) {
            return value;
        }
        return undefined;
    }

    function resolveRows(): number {
        if (isNotNil(rows)) {
            return rows;
        }
        return 8;
    }

    function resolveRootClassName(): string {
        const base = mode === 'preview' ? 'tm-agent-md-field is-preview' : 'tm-agent-md-field is-edit';
        if (isNotEmpty(className)) {
            return `${base} ${className}`;
        }
        return base;
    }

    function handleModeChange(next: MarkdownFieldMode): void {
        setMode(next);
    }

    function handleTextChange(next: string): void {
        onChange(next);
    }

    return (
        <Box className={resolveRootClassName()}>
            <Box className="tm-agent-md-field-head">
                {isNotNil(label) ? (
                    <Typography className="tm-agent-md-field-label">{label}</Typography>
                ) : <Box className="tm-agent-md-field-label-spacer" />}
                <Box className="tm-agent-md-field-modes">
                    <ButtonBase disableRipple aria-pressed={mode === 'preview'} onClick={() => handleModeChange('preview')} className={mode === 'preview' ? 'tm-agent-md-field-mode is-active' : 'tm-agent-md-field-mode'}>
                        {t('agent.markdown.preview')}
                    </ButtonBase>
                    <ButtonBase disableRipple aria-pressed={mode === 'edit'} onClick={() => handleModeChange('edit')} className={mode === 'edit' ? 'tm-agent-md-field-mode is-active' : 'tm-agent-md-field-mode'}>
                        {t('agent.markdown.edit')}
                    </ButtonBase>
                </Box>
            </Box>
            {mode === 'preview' ? (
                <Box className="tm-agent-md-field-preview">
                    {isEmpty(resolveContent()) ? (
                        <Typography className="tm-agent-md-field-empty">{placeholder}</Typography>
                    ) : (
                        <AgentMarkdown content={resolveContent()} className="tm-agent-md"/>
                    )}
                </Box>
            ) : (
                <Box className="tm-agent-md-field-editor">
                    <TextAreaField value={resolveTextAreaValue()} onChange={handleTextChange} placeholder={placeholder} rows={resolveRows()} />
                </Box>
            )}
        </Box>
    );
}
