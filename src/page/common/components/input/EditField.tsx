import type { ReactNode } from 'react';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { useEffect, useRef, useState } from 'react';
import { isNotNil } from '@/common/util/StringUtil.ts';
import { Visibility, VisibilityOff } from '@mui/icons-material';
import { Box, ButtonBase, InputBase, Typography } from '@mui/material';

export function EditField(props: { label?: string; value: string | undefined | null; onChange: (value: string) => void; onBlur?: () => void; placeholder?: string; password?: boolean; labelWidth?: number; autoFocus?: boolean; actionIcon?: ReactNode; actionLabel?: string; onAction?: () => void }) {
  const { t } = useI18n();
  const [revealed, setRevealed] = useState<boolean>(false);
  const inputRef = useRef<HTMLInputElement | null>(null);
  const { label, value, onChange, onBlur, placeholder, password, labelWidth, autoFocus, actionIcon, actionLabel, onAction } = props;

  const resolveLabelWidth = (): number => {
    if (isNotNil(labelWidth)) {
      return labelWidth;
    }
    return 64;
  };

  const isInputMasked = (): boolean => {
    return password === true && !revealed;
  };

  const resolveInputType = (): string => {
    if (isInputMasked()) {
      return 'password';
    }
    return 'text';
  };

  const resolveInputValue = (): string => {
    if (isNotNil(value)) {
      return value;
    }
    return '';
  };

  const handleToggleReveal = (): void => {
    setRevealed(!revealed);
  };

  useEffect(() => {
    if (autoFocus !== true) {
      return;
    }
    inputRef.current?.focus({ preventScroll: true });
  }, [autoFocus]);

  return (
    <Box
      className="tm-edit-field"
      sx={{ display: 'flex', alignItems: 'center', width: '100%', maxWidth: '100%', minWidth: 0, flexShrink: 0, boxSizing: 'border-box', minHeight: '38px', border: '1px solid var(--tm-border-subtle)', borderRadius: '7px', bgcolor: 'transparent', transition: 'border-color 120ms ease, background-color 120ms ease', '&:hover': { bgcolor: 'var(--tm-bg-hover)' }, '&:focus-within': { bgcolor: 'var(--tm-bg-hover)', borderColor: 'var(--tm-border-accent)' }}}
    >
      {isNotNil(label) ? (
        <Typography className="tm-edit-field-label" sx={{ fontSize: 13, color: 'var(--tm-text-tertiary)', pl: 1, width: resolveLabelWidth(), flexShrink: 0, whiteSpace: 'nowrap', userSelect: 'none' }}>
          {label}
        </Typography>
      ) : null}
      <InputBase
        className="tm-edit-field-input"
        inputRef={inputRef}
        type={resolveInputType()}
        value={resolveInputValue()}
        onChange={e => onChange(e.target.value)}
        onBlur={onBlur}
        placeholder={placeholder}
        sx={{ flex: 1, minWidth: 0, px: 1, py: 0.5, fontSize: 13, color: 'var(--tm-text-primary)', '& input': { minWidth: 0 }, '& input::placeholder': { color: 'var(--tm-text-tertiary)', opacity: 1 }, '& input::-ms-reveal': { display: 'none' }, '& input::-ms-clear': { display: 'none' } }}
      />
      {password === true ? (
        <ButtonBase className="tm-edit-field-action" onClick={handleToggleReveal} aria-label={revealed ? t('home.keys.hide') : t('home.keys.show')}>
          {revealed ? <VisibilityOff /> : <Visibility />}
        </ButtonBase>
      ) : isNotNil(onAction) ? (
        <ButtonBase className="tm-edit-field-action" onClick={onAction} aria-label={actionLabel}>
          {actionIcon}
        </ButtonBase>
      ) : null}
    </Box>
  );
}
