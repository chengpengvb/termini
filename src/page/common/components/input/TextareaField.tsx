import { codeToHtml } from 'shiki';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { useEffect, useRef, useState } from 'react';
import { CloseFullscreen, OpenInFull } from '@mui/icons-material';
import { isNotEmpty, isNotNil } from '@/common/util/StringUtil.ts';
import { Box, ButtonBase, InputBase, Typography } from '@mui/material';
import { usePlatformMobile } from '@/common/context/usePlatformMobile';
import { HIGHLIGHT_OPTIONS } from '@/page/common/util/codeHighlight.ts';

const MONO_FONT = 'var(--tm-font-mono, ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace)';

export function TextareaField(props: { label?: string; minRows?: number; maxRows?: number; language?: string; labelWidth?: number; maxLength?: number; value: string | null; placeholder?: string; expandable?: boolean; autoFocus?: boolean; onBlur?: () => void; onChange: (value: string) => void }) {
  const { t } = useI18n();
  const { value } = props;
  const mobile = usePlatformMobile();
  const canExpand = props.expandable === true;
  const inputValue = isNotNil(value) ? value : '';
  const [expanded, setExpanded] = useState(false);
  const isExpanded = canExpand && expanded;
  const inputRef = useRef<HTMLInputElement | null>(null);
  const [html, setHtml] = useState<string | null>(null);
  const highlightRef = useRef<HTMLDivElement | null>(null);
  const hasHighlight = isNotEmpty(props.language) && !mobile;
  const { label, onChange, onBlur, placeholder, labelWidth, minRows, maxRows, maxLength, language, autoFocus } = props;

  const resolveLabelWidth = (): number => {
    if (isNotNil(labelWidth)) {
      return labelWidth;
    }
    return 64;
  };

  const resolveMinRows = (): number => {
    if (isExpanded) {
      return 10;
    }
    if (isNotNil(minRows)) {
      return minRows;
    }
    return 3;
  };

  const resolveMaxRows = (): number => {
    if (isExpanded) {
      return 20;
    }
    if (isNotNil(maxRows)) {
      return maxRows;
    }
    return 8;
  };

  const handleToggleExpand = (): void => {
    setExpanded(!expanded);
  };

  const handleScroll = (event: { currentTarget: HTMLElement }): void => {
    const el = highlightRef.current;
    if (!el) {
      return;
    }
    el.scrollTop = event.currentTarget.scrollTop;
    el.scrollLeft = event.currentTarget.scrollLeft;
  };

  useEffect(() => {
    if (!hasHighlight) {
      setHtml(null);
      return;
    }
    let cancelled = false;
    const lang = language as string;
    const timer = window.setTimeout(() => {
      codeToHtml(inputValue, { lang, ...HIGHLIGHT_OPTIONS })
        .then(result => {
          if (!cancelled) {
            setHtml(result);
          }
        })
        .catch(() => {
          codeToHtml(inputValue, { lang: 'text', ...HIGHLIGHT_OPTIONS }).then(result => {
            if (!cancelled) {
              setHtml(result);
            }
          });
        });
    }, 80);
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [hasHighlight, inputValue, language]);

  useEffect(() => {
    if (!isExpanded) {
      return;
    }
    inputRef.current?.focus({ preventScroll: true });
  }, [isExpanded]);

  useEffect(() => {
    if (autoFocus !== true) {
      return;
    }
    inputRef.current?.focus({ preventScroll: true });
  }, [autoFocus]);

  const highlightLayerSx = {
    position: 'absolute' as const,
    inset: 0,
    overflow: 'hidden',
    pointerEvents: 'none' as const,
    '& pre': {
      margin: 0,
      padding: isExpanded ? '6px 36px 6px 8px' : '4px 8px',
      background: 'transparent !important',
      whiteSpace: 'pre-wrap',
      wordBreak: 'break-word',
      overflowWrap: 'anywhere',
      fontFamily: MONO_FONT,
      fontSize: 13,
      lineHeight: 1.5,
    },
    '& code': {
      fontFamily: 'inherit !important',
      fontSize: 'inherit !important',
      lineHeight: 'inherit !important',
      background: 'transparent !important',
    },
  };

  const inputSx = {
    flex: 1,
    minWidth: 0,
    position: 'relative' as const,
    zIndex: 1,
    px: hasHighlight ? 0 : 1,
    py: hasHighlight ? 0 : 0.5,
    fontSize: 13,
    color: hasHighlight ? 'transparent' : 'var(--tm-text-primary)',
    fontFamily: MONO_FONT,
    '& textarea': {
      width: '100%',
      overflowWrap: 'anywhere',
      wordBreak: 'break-word',
      ...(hasHighlight
        ? {
            color: 'transparent',
            WebkitTextFillColor: 'transparent',
            caretColor: 'var(--tm-text-primary)',
            px: isExpanded ? '8px' : '8px',
            pr: isExpanded ? '36px' : '8px',
            py: isExpanded ? '6px' : '4px',
          }
        : {}),
    },
    '& textarea::placeholder': {
      color: 'var(--tm-text-tertiary)',
      opacity: 1,
      WebkitTextFillColor: 'var(--tm-text-tertiary)',
    },
  };

  const renderEditor = () => (
    <Box sx={{ position: 'relative', display: 'flex', flex: 1, minWidth: 0, width: '100%', alignSelf: 'stretch' }}>
      {hasHighlight && isNotNil(html) ? <Box ref={highlightRef} aria-hidden sx={highlightLayerSx} dangerouslySetInnerHTML={{ __html: html }} /> : null}
      <InputBase
        inputRef={inputRef}
        multiline
        fullWidth
        minRows={resolveMinRows()}
        maxRows={resolveMaxRows()}
        value={inputValue}
        onChange={e => onChange(e.target.value)}
        onBlur={onBlur}
        placeholder={placeholder}
        inputProps={{ spellCheck: false, onScroll: handleScroll, ...(isNotNil(maxLength) ? { maxLength } : {}) }}
        sx={inputSx}
      />
    </Box>
  );

  if (isExpanded) {
    return (
      <Box
        className="tm-edit-field tm-edit-field-textarea is-expanded"
        sx={{ position: 'relative', display: 'flex', width: '100%', maxWidth: '100%', minWidth: 0, boxSizing: 'border-box', border: '1px solid var(--tm-border-subtle)', borderRadius: '7px', bgcolor: 'var(--tm-bg-app)', transition: 'border-color 120ms ease, background-color 120ms ease', '&:hover': { bgcolor: 'var(--tm-bg-hover)' }, '&:focus-within': { bgcolor: 'var(--tm-bg-hover)', borderColor: 'var(--tm-border-accent)' }}}
      >
        {renderEditor()}
        <ButtonBase className="tm-edit-field-action tm-edit-field-action-float" disableRipple onClick={handleToggleExpand} aria-label={t('common.collapseEdit')}>
          <CloseFullscreen />
        </ButtonBase>
      </Box>
    );
  }

  return (
    <Box
      className="tm-edit-field tm-edit-field-textarea"
      sx={{ display: 'flex', alignItems: canExpand ? 'center': 'flex-start', width: '100%', maxWidth: '100%', minWidth: 0, boxSizing: 'border-box', minHeight: '38px', border: '1px solid var(--tm-border-subtle)', borderRadius: '7px', bgcolor: 'transparent', transition: 'border-color 120ms ease, background-color 120ms ease', '&:hover': { bgcolor: 'var(--tm-bg-hover)' }, '&:focus-within': { bgcolor: 'var(--tm-bg-hover)', borderColor: 'var(--tm-border-accent)' }}}
    >
      {isNotNil(label) ? (
        <Typography className="tm-edit-field-label" sx={{ fontSize: 13, color: 'var(--tm-text-tertiary)', pl: 1, pt: canExpand ? 0 : 0.75, width: resolveLabelWidth(), flexShrink: 0, whiteSpace: 'nowrap', userSelect: 'none' }}>
          {label}
        </Typography>
      ) : null}
      {renderEditor()}
      {canExpand ? (
        <ButtonBase className="tm-edit-field-action tm-edit-field-action-outline" disableRipple onClick={handleToggleExpand} aria-label={t('common.expandEdit')}>
          <OpenInFull />
        </ButtonBase>
      ) : null}
    </Box>
  );
}
