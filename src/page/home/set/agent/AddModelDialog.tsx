import { useI18n } from '@/common/i18n/useI18n.ts';
import { useEffect, useRef, useState } from 'react';
import { Check, Close, Search } from '@mui/icons-material';
import type { KeyboardEvent, MouseEvent, ReactNode } from 'react';
import { useAiProvideService } from '@/common/context/useAppHooks.tsx';
import { useBackDismiss } from '@/page/common/hooks/useBackDismiss.ts';
import { usePlatformMobile } from '@/common/context/usePlatformMobile';
import { EditField } from '@/page/common/components/input/EditField.tsx';
import { isEmpty, isNotEmpty, isNotNil } from '@/common/util/StringUtil.ts';
import { SelectField } from '@/page/common/components/input/SelectField.tsx';
import { SwitchField } from '@/page/common/components/input/SwitchField.tsx';
import { useOpenTransition } from '@/page/common/hooks/useOpenTransition.ts';
import { ActionSheet } from '@/page/common/components/dialog/ActionSheet.tsx';
import type { RespModel, RespProvider } from '@/common/model/resp/RespProvider.ts';
import { AI_PROVIDE_KIND, AI_PROVIDE_SOURCE, type AiProvide } from '@/common/model/AiProvide.ts';
import { BizError, RESULT_CODE, resolveResultMessage } from '@/common/model/resp/ResultModel.ts';
import { Box, Button, ButtonBase, CircularProgress, InputBase, Menu, MenuItem, MenuList, Typography } from '@mui/material';

interface AddModelDialogProps {
  open: boolean;
  onClose: () => void;
  editing: AiProvide | null;
  templateList: RespProvider[];
}

export function AddModelDialog(props: AddModelDialogProps) {
  const { t } = useI18n();
  const mobileViewport = usePlatformMobile();
  const aiProvideService = useAiProvideService();
  const [saving, setSaving] = useState<boolean>(false);
  const { open, editing, templateList, onClose } = props;
  const { mounted, active } = useOpenTransition(open, 360);
  const [errorCode, setErrorCode] = useState<number | null>(null);
  const [anchorEl, setAnchorEl] = useState<HTMLElement | null>(null);
  const searchInputRef = useRef<HTMLInputElement | null>(null);
  const [provide, setProvide] = useState<AiProvide>({
    url: null,
    key: null,
    model: null,
    provider: null,
    max_token: null,
    kind: AI_PROVIDE_KIND.CHAT,
    vision: false,
    source: AI_PROVIDE_SOURCE.CUSTOM,
  });

  const createEmptyProvide = (): AiProvide => {
    return {
      url: null,
      key: null,
      model: null,
      provider: null,
      max_token: null,
      kind: AI_PROVIDE_KIND.CHAT,
      vision: false,
      source: AI_PROVIDE_SOURCE.CUSTOM,
    };
  };

  const resolveCreateProvide = (): AiProvide => {
    if (templateList.length === 0) {
      return createEmptyProvide();
    }
    const first = templateList[0];
    if (isNotEmpty(first.default_url)) {
      return {
        url: first.default_url,
        key: null,
        model: null,
        max_token: null,
        kind: AI_PROVIDE_KIND.CHAT,
        vision: false,
        provider: first.provide_id,
        source: AI_PROVIDE_SOURCE.CUSTOM,
      };
    }
    return {
      url: null,
      key: null,
      model: null,
      max_token: null,
      kind: AI_PROVIDE_KIND.CHAT,
      vision: false,
      provider: first.provide_id,
      source: AI_PROVIDE_SOURCE.CUSTOM,
    };
  };

  const isEditMode = (): boolean => {
    return isNotEmpty(provide.id);
  };

  const resolveDialogTitle = (): string => {
    if (isEditMode()) {
      return t('set.model.edit');
    }
    return t('set.model.add');
  };

  const resolveTemplate = (): RespProvider | null => {
    if (!isNotNil(provide.provider)) {
      return null;
    }
    const found = templateList.find(item => item.provide_id === provide.provider);
    if (!isNotNil(found)) {
      return null;
    }
    return found;
  };

  const resolveProviderOptions = () => {
    return templateList.map(item => ({ value: String(item.provide_id), label: item.name }));
  };

  const resolveProviderSelectValue = (): string | null => {
    if (isNotNil(provide.provider)) {
      return String(provide.provider);
    }
    return null;
  };

  const resolveModelList = (): RespModel[] => {
    const template = resolveTemplate();
    if (!isNotNil(template)) {
      return [];
    }
    return template.models;
  };

  const resolveFilteredModels = (): RespModel[] => {
    const list = resolveModelList();
    if (!isNotEmpty(provide.model)) {
      return list;
    }
    const keyword = provide.model.trim().toLowerCase();
    if (list.some(model => model.name.toLowerCase() === keyword)) {
      return list;
    }
    return list.filter(model => model.name.toLowerCase().includes(keyword));
  };

  const resolveModelInputValue = (): string => {
    if (isNotEmpty(provide.model)) {
      return provide.model;
    }
    return '';
  };

  const resolveMaxTokenValue = (): string | null => {
    if (isNotNil(provide.max_token)) {
      return String(provide.max_token);
    }
    return null;
  };

  const resolveKindSelectValue = (): string => {
    if (isNotNil(provide.kind)) {
      return String(provide.kind);
    }
    return String(AI_PROVIDE_KIND.CHAT);
  };

  const resolveKindOptions = () => {
    return [
      { value: String(AI_PROVIDE_KIND.CHAT), label: t('set.model.kindChat') },
      { value: String(AI_PROVIDE_KIND.EMBEDDING), label: t('set.model.kindEmbedding') },
      { value: String(AI_PROVIDE_KIND.IMAGE_GEN), label: t('set.model.kindImageGen') },
      { value: String(AI_PROVIDE_KIND.VIDEO), label: t('set.model.kindVideo') },
    ];
  };

  const resolveModelTriggerClass = (): string => {
    if (isNotEmpty(provide.model)) {
      return 'tm-set-add-model-field-trigger';
    }
    return 'tm-set-add-model-field-trigger is-empty';
  };

  const resolveModelItemClass = (name: string): string => {
    if (isNotEmpty(provide.model) && name === provide.model) {
      return 'tm-menu-item is-selected';
    }
    return 'tm-menu-item';
  };

  const renderModelCheck = (name: string): ReactNode => {
    if (isNotEmpty(provide.model) && name === provide.model) {
      return <Check />;
    }
    return null;
  };

  const renderModelDisplay = (): ReactNode => {
    if (isNotEmpty(provide.model)) {
      return provide.model;
    }
    return t('set.model.namePlaceholder');
  };

  const handleProviderChange = (raw: string): void => {
    const next = Number(raw);
    const template = templateList.find(item => item.provide_id === next);
    if (isNotNil(template) && isNotEmpty(template.default_url)) {
      setProvide({
        ...provide,
        provider: next,
        model: null,
        max_token: null,
        vision: false,
        url: template.default_url,
      });
      return;
    }
    setProvide({
      ...provide,
      provider: next,
      model: null,
      max_token: null,
      vision: false,
      url: null,
    });
  };

  const handleUrlChange = (next: string): void => {
    if (isEmpty(next)) {
      setProvide({
        ...provide,
        url: null,
      });
      return;
    }
    setProvide({
      ...provide,
      url: next,
    });
  };

  const handleApiKeyChange = (next: string): void => {
    if (isEmpty(next)) {
      setProvide({
        ...provide,
        key: null,
      });
      return;
    }
    setProvide({
      ...provide,
      key: next,
    });
  };

  const handleOpenModelMenu = (e: MouseEvent<HTMLElement>): void => {
    if (!isNotNil(provide.provider)) {
      return;
    }
    setAnchorEl(e.currentTarget);
  };

  const handleCloseModelMenu = (): void => {
    setAnchorEl(null);
  };

  const handleModelInputChange = (raw: string): void => {
    setProvide({
      ...provide,
      model: isEmpty(raw) ? null : raw,
    });
  };

  const handleKindChange = (raw: string): void => {
    setProvide({
      ...provide,
      kind: Number(raw),
    });
  };

  const handleMaxTokenChange = (raw: string): void => {
    if (isEmpty(raw)) {
      setProvide({
        ...provide,
        max_token: null,
      });
      return;
    }
    const next = Number(raw);
    if (!Number.isInteger(next) || next <= 0) {
      return;
    }
    setProvide({
      ...provide,
      max_token: next,
    });
  };

  const handlePickModel = (name: string): void => {
    const template = templateList.find(item => item.provide_id === provide.provider);
    const model = template?.models.find(item => item.name === name);
    setProvide({
      ...provide,
      model: name,
      max_token: isNotNil(model) ? model.max_token : provide.max_token,
      vision: isNotNil(model) ? model.vision === true : provide.vision === true,
    });
    handleCloseModelMenu();
  };

  const handleVisionChange = (next: boolean): void => {
    setProvide({
      ...provide,
      vision: next,
    });
  };

  const canSave = (): boolean => {
    return !saving && isNotNil(provide.provider) && isNotEmpty(provide.url) && isNotEmpty(provide.model) && isNotNil(provide.max_token);
  };

  const resetForm = (): void => {
    setProvide(createEmptyProvide());
    setAnchorEl(null);
    setErrorCode(null);
  };

  const handleClose = (): void => {
    resetForm();
    onClose();
  };

  const resolveErrorMessage = (code: number): string => {
    return resolveResultMessage(code);
  };

  const handleSaveError = (error: unknown): void => {
    if (error instanceof BizError) {
      setErrorCode(error.code);
      return;
    }
    setErrorCode(RESULT_CODE.ERROR);
  };

  const renderErrorTip = (): ReactNode => {
    if (!isNotNil(errorCode)) {
      return null;
    }
    return <Typography className="tm-set-add-model-error">{resolveErrorMessage(errorCode)}</Typography>;
  };

  const renderSaveStartIcon = (): ReactNode => {
    if (!saving) {
      return null;
    }
    return <CircularProgress size={14} color="inherit" />;
  };

  const resolveModelSheetItemClass = (name: string): string => {
    if (isNotEmpty(provide.model) && provide.model === name) {
      return 'tm-action-sheet-item is-selected';
    }
    return 'tm-action-sheet-item';
  };

  const renderModelMenuList = (mobile: boolean): ReactNode => {
    const list = resolveFilteredModels();
    if (list.length === 0) {
      return <Box className={mobile ? 'tm-action-sheet-empty' : 'tm-set-add-model-menu-empty'}>{t('set.model.nameEmpty')}</Box>;
    }
    return list.map(model => (
      <MenuItem key={model.name} className={mobile ? resolveModelSheetItemClass(model.name) : resolveModelItemClass(model.name)} onClick={() => handlePickModel(model.name)}>
        <Box className="tm-menu-item-icon">{renderModelCheck(model.name)}</Box>
        <Box className="tm-menu-item-text">{model.name}</Box>
      </MenuItem>
    ));
  };

  const renderModelPicker = (): ReactNode => {
    const modelMenuOpen = isNotNil(anchorEl);
    const searchRow = (
      <Box className={mobileViewport ? 'tm-action-sheet-search' : 'tm-set-add-model-search'} onKeyDown={handleSearchKeyDown}>
        <Search className={mobileViewport ? undefined : 'tm-set-add-model-search-icon'} sx={mobileViewport ? { fontSize: 18, color: 'var(--tm-text-tertiary)', flexShrink: 0 } : undefined} />
        <InputBase
          inputRef={searchInputRef}
          value={resolveModelInputValue()}
          onChange={e => handleModelInputChange(e.target.value)}
          placeholder={t('set.model.nameSearch')}
          className={mobileViewport ? undefined : 'tm-set-add-model-search-input'}
          sx={mobileViewport ? { flex: 1, fontSize: 16, color: 'var(--tm-text-primary)', '& input::placeholder': { color: 'var(--tm-text-tertiary)', opacity: 1 } } : undefined}
        />
      </Box>
    );
    if (mobileViewport) {
      return (
        <ActionSheet open={modelMenuOpen} onClose={handleCloseModelMenu}>
          {searchRow}
          <MenuList disablePadding className="tm-action-sheet-list">
            {renderModelMenuList(true)}
          </MenuList>
        </ActionSheet>
      );
    }
    return (
      <Menu
        open={modelMenuOpen}
        anchorEl={anchorEl}
        onClose={handleCloseModelMenu}
        anchorOrigin={{ vertical: 'bottom', horizontal: 'left' }}
        transformOrigin={{ vertical: 'top', horizontal: 'left' }}
        slotProps={{ root: { onMouseDown: (e: MouseEvent) => e.stopPropagation() }, paper: { className: 'tm-menu-paper tm-set-add-model-menu-paper' }, list: { className: 'tm-menu-list' } }}
      >
        {searchRow}
        <Box className="tm-set-add-model-menu-list">{renderModelMenuList(false)}</Box>
      </Menu>
    );
  };

  const finishSaveSuccess = (): void => {
    setSaving(false);
    resetForm();
    onClose();
  };

  const handleSave = (): void => {
    if (!canSave()) {
      return;
    }
    setSaving(true);
    setErrorCode(null);
    if (isEditMode()) {
      aiProvideService
        .update(provide)
        .then(finishSaveSuccess)
        .catch(error => {
          setSaving(false);
          handleSaveError(error);
        });
      return;
    }
    aiProvideService
      .insert(provide)
      .then(finishSaveSuccess)
      .catch(error => {
        setSaving(false);
        handleSaveError(error);
      });
  };

  const handleOverlayMouseDown = (e: MouseEvent<HTMLElement>): void => {
    if (e.target !== e.currentTarget) {
      return;
    }
    handleClose();
  };

  const handlePaperMouseDown = (e: MouseEvent<HTMLElement>): void => {
    e.stopPropagation();
  };

  const handleSearchKeyDown = (e: KeyboardEvent<HTMLElement>): void => {
    e.stopPropagation();
    if (e.key !== 'Enter' && e.key !== 'Escape') {
      return;
    }
    e.preventDefault();
    handleCloseModelMenu();
  };

  useBackDismiss(open, onClose, 'add-model');

  useEffect(() => {
    if (!open) {
      return;
    }
    setErrorCode(null);
    setAnchorEl(null);
    if (editing) {
      setProvide(editing);
      return;
    }
    setProvide(resolveCreateProvide());
  }, [open, templateList, editing]);

  useEffect(() => {
    if (!isNotNil(anchorEl)) {
      return;
    }
    const timer = window.setTimeout(() => {
      searchInputRef.current?.focus({ preventScroll: true });
    }, 32);
    return () => {
      window.clearTimeout(timer);
    };
  }, [anchorEl]);

  if (!mounted) {
    return null;
  }

  return (
    <Box className={active ? 'tm-set-add-model-overlay is-open' : 'tm-set-add-model-overlay'} onMouseDown={handleOverlayMouseDown}>
      <Box className="tm-set-add-model-paper" onMouseDown={handlePaperMouseDown}>
        <Box className="tm-set-add-model-header">
          <Typography className="tm-set-add-model-title">{resolveDialogTitle()}</Typography>
          <ButtonBase disableRipple onClick={handleClose} aria-label={t('common.close')} className="tm-set-add-model-close">
            <Close className="tm-set-add-model-close-icon" />
          </ButtonBase>
        </Box>
        <Box className="tm-set-add-model-body">
          <SelectField label={t('set.model.provider')} labelWidth={80} value={resolveProviderSelectValue()} onChange={handleProviderChange} options={resolveProviderOptions()} placeholder={t('set.model.providerPlaceholder')} />
          <EditField label="Base URL" labelWidth={80} value={provide.url} onChange={handleUrlChange} placeholder={t('set.model.baseUrlPlaceholder')} />
          <EditField label="API Key" labelWidth={80} value={provide.key} onChange={handleApiKeyChange} placeholder={t('set.model.apiKeyPlaceholder')} password />
          <Box className="tm-set-add-model-field">
            <Typography className="tm-set-add-model-field-label">{t('set.model.name')}</Typography>
            <ButtonBase disableRipple onClick={handleOpenModelMenu} className={resolveModelTriggerClass()}>
              <Box className="tm-set-add-model-field-value">{renderModelDisplay()}</Box>
            </ButtonBase>
          </Box>
          <EditField label={t('set.model.maxToken')} labelWidth={80} value={resolveMaxTokenValue()} onChange={handleMaxTokenChange} placeholder={t('set.model.maxTokenPlaceholder')} />
          <SelectField label={t('set.model.kind')} labelWidth={80} value={resolveKindSelectValue()} onChange={handleKindChange} options={resolveKindOptions()} />
          <SwitchField label={t('set.model.vision')} value={provide.vision === true} onChange={handleVisionChange} description={t('set.model.visionHint')} />
        </Box>
        <Box className="tm-set-add-model-footer tm-edit-dialog-footer">
          <Button disableRipple onClick={handleClose} className="tm-btn tm-btn-secondary">
            {t('common.cancel')}
          </Button>
          <Box className="tm-set-add-model-footer-spacer tm-edit-dialog-footer-spacer" />
          {renderErrorTip()}
          <Button disableRipple disabled={!canSave()} onClick={handleSave} startIcon={renderSaveStartIcon()} className="tm-btn tm-btn-primary">
            {t('common.save')}
          </Button>
        </Box>
      </Box>
      {renderModelPicker()}
    </Box>
  );
}
