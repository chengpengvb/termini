import { useI18n } from '@/common/i18n/useI18n.ts';
import { ArrowDropDown } from '@mui/icons-material';
import { Box, ButtonBase, Popover, Typography } from '@mui/material';
import { useAiProvideService } from '@/common/context/useAppHooks.tsx';
import { usePlatformMobile } from '@/common/context/usePlatformMobile';
import { isEmpty, isNotEmpty, isNotNil } from '@/common/util/StringUtil.ts';
import { OverlayPopoverPaper } from '@/page/common/util/scopedDialogUtil.ts';
import { ActionSheet } from '@/page/common/components/dialog/ActionSheet.tsx';
import { useEffect, useMemo, useRef, useState, startTransition } from 'react';
import { useAiProvideOptions } from '@/page/common/hooks/useAiProvideOptions.ts';
import { ModelSelectMenu } from '@/page/common/components/model/ModelSelectMenu.tsx';
import { AI_PROVIDE_KIND, resolveAiProvideModelLabel, type AiProvide } from '@/common/model/AiProvide.ts';

interface ModelSelectProps {
  label?: string;
  labelWidth?: number;
  autoSelect?: boolean;
  selectedId: string | null;
  variant?: 'toolbar' | 'row';
  onSelect: (provide: AiProvide) => void;
}

export function ModelSelect(props: ModelSelectProps) {
  const { t } = useI18n();
  const onSelectRef = useRef(props.onSelect);
  const mobileViewport = usePlatformMobile();
  const aiProvideService = useAiProvideService();
  const [open, setOpen] = useState<boolean>(false);
  const rowRef = useRef<HTMLDivElement | null>(null);
  const buttonRef = useRef<HTMLButtonElement | null>(null);
  const { cloudList, customList, templateList, refreshing, refresh } = useAiProvideOptions();
  const chatCustomList = useMemo(() => customList.filter(item => item.kind === AI_PROVIDE_KIND.CHAT), [customList]);
  const allList = useMemo(() => [...cloudList, ...chatCustomList], [cloudList, chatCustomList]);
  const { selectedId, onSelect, variant, autoSelect, label, labelWidth } = props;

  onSelectRef.current = onSelect;

  function isRowVariant(): boolean {
    return variant === 'row';
  }

  function resolveAnchorEl(): HTMLElement | null {
    if (isRowVariant()) {
      return rowRef.current;
    }
    return buttonRef.current;
  }

  function resolveLabelWidth(): number {
    if (isNotNil(labelWidth)) {
      return labelWidth;
    }
    return 64;
  }

  function resolveSelected(): AiProvide | null {
    if (isEmpty(selectedId)) {
      return null;
    }
    const found = allList.find(item => item.id === selectedId);
    if (isNotNil(found)) {
      return found;
    }
    return null;
  }

  function resolveTriggerLabel(): string {
    const selected = resolveSelected();
    if (isNotNil(selected)) {
      return resolveAiProvideModelLabel(selected, templateList);
    }
    if (allList.length === 0) {
      if (refreshing) {
        return t('common.modelLoading');
      }
      return t('common.modelNotConfigured');
    }
    return t('common.selectModel');
  }

  function handleOpen(): void {
    setOpen(true);
    refresh();
  }

  function handleClose(): void {
    setOpen(false);
  }

  function handlePick(provide: AiProvide): void {
    handleClose();
    startTransition(() => {
      onSelectRef.current(provide);
    });
  }

  useEffect(() => {
    if (autoSelect !== true || refreshing || allList.length === 0) {
      return;
    }
    if (isNotEmpty(selectedId) && allList.some(item => item.id === selectedId)) {
      return;
    }
    const remembered = aiProvideService.getSelectedProvide();
    const found = isNotNil(remembered) ? allList.find(item => item.id === remembered.id && item.source === remembered.source) : undefined;
    const next = found ?? allList[0];
    if (!next || !next.id) {
      return;
    }
    aiProvideService.setSelectedProvide(next.id, next.source);
    onSelectRef.current(next);
  }, [allList, selectedId, aiProvideService, autoSelect, refreshing]);

  const triggerLabel = resolveTriggerLabel();
  const triggerClassName = ['tm-agent-model-trigger', allList.length === 0 ? 'is-empty' : '', refreshing ? 'is-loading' : ''].filter(Boolean).join(' ');

  return (
    <>
      {isRowVariant() ? (
        <Box
          ref={rowRef}
          onClick={handleOpen}
          className="tm-edit-field"
          sx={{ display: 'flex', alignItems: 'center', width: '100%', maxWidth: '100%', minWidth: 0, flexShrink: 0, boxSizing: 'border-box', minHeight: '38px', cursor: 'pointer', border: '1px solid var(--tm-border-subtle)', borderRadius: '7px', bgcolor: 'transparent', transition: 'border-color 120ms ease, background-color 120ms ease', '&:hover': { bgcolor: 'var(--tm-bg-hover)' }}}
        >
          {isNotNil(label) ? (
            <Typography className="tm-edit-field-label" sx={{ fontSize: 13, color: 'var(--tm-text-tertiary)', pl: 1, width: resolveLabelWidth(), flexShrink: 0, whiteSpace: 'nowrap', userSelect: 'none' }}>
              {label}
            </Typography>
          ) : null}
          <Box className="tm-edit-field-value" sx={{ flex: 1, minWidth: 0, px: 1, py: 0.5, fontSize: 13, color: allList.length === 0 ? 'var(--tm-text-tertiary)' : 'var(--tm-text-primary)', overflow: 'clip', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
            {triggerLabel}
          </Box>
          <ArrowDropDown className={open ? 'tm-edit-field-caret is-open' : 'tm-edit-field-caret'} />
        </Box>
      ) : (
        <ButtonBase ref={buttonRef} disableRipple onClick={handleOpen} aria-label={triggerLabel} title={triggerLabel} className={triggerClassName}>
          <Typography component="span" className="tm-agent-model-trigger-label" title={triggerLabel}>
            {triggerLabel}
          </Typography>
          <ArrowDropDown className="tm-agent-model-trigger-caret" />
        </ButtonBase>
      )}
      {mobileViewport ? (
        <ActionSheet open={open} onClose={handleClose}>
          <ModelSelectMenu selected={resolveSelected()} cloudList={cloudList} customList={chatCustomList} templateList={templateList} refreshing={refreshing} mobileViewport={mobileViewport} onPick={handlePick} />
        </ActionSheet>
      ) : (
        <Popover
          open={open}
          anchorEl={resolveAnchorEl()}
          onClose={handleClose}
          marginThreshold={8}
          anchorOrigin={isRowVariant() ? { vertical: 'bottom', horizontal: 'left' } : { vertical: 'top', horizontal: 'left' }}
          transformOrigin={isRowVariant() ? { vertical: 'top', horizontal: 'left' } : { vertical: 'bottom', horizontal: 'left' }}
          slots={{ paper: OverlayPopoverPaper }}
          slotProps={{ paper: { className: 'tm-agent-model-menu-paper' } }}
        >
          <ModelSelectMenu selected={resolveSelected()} cloudList={cloudList} customList={chatCustomList} templateList={templateList} refreshing={refreshing} mobileViewport={mobileViewport} onPick={handlePick} />
        </Popover>
      )}
    </>
  );
}
