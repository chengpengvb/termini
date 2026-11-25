import type { MouseEvent } from 'react';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { useEffect, useRef, useState } from 'react';
import { EditDialog } from '../../../../keys/edit/EditDialog';
import { isEmpty, isNotEmpty } from '@/common/util/StringUtil';
import type { ConfigKeyPair } from '@/common/model/ConfigKeyPair.ts';
import { resolveErrorText } from '@/common/model/resp/ResultModel.ts';
import { useConfigKeyPairService } from '@/common/context/useAppHooks';
import { usePlatformMobile } from '@/common/context/usePlatformMobile';
import { AlertDialog } from '@/common/components/dialog/AlertDialog.tsx';
import { createEmptyConfigKeyPair } from '@/common/model/ConfigKeyPair.ts';
import { ActionSheet } from '@/page/common/components/dialog/ActionSheet.tsx';
import { Add, Check, DriveFileRenameOutline, Search, VpnKey } from '@mui/icons-material';
import { Box, ButtonBase, InputBase, Menu, MenuItem, MenuList, Typography } from '@mui/material';

interface Draft {
  isNew: boolean;
  keyPair: ConfigKeyPair;
}

export function KeyPairFields(props: { value?: string; onChange: (keyPairId: string) => void }) {
  const { t, tf } = useI18n();
  const { value, onChange } = props;
  const mobileViewport = usePlatformMobile();
  const [draft, setDraft] = useState<Draft | null>(null);
  const configKeyPairService = useConfigKeyPairService();
  const searchInputRef = useRef<HTMLInputElement | null>(null);
  const [keyPairs, setKeyPairs] = useState<ConfigKeyPair[]>([]);
  const [searchText, setSearchText] = useState<string | null>(null);
  const [anchorEl, setAnchorEl] = useState<HTMLElement | null>(null);
  const [alertInfo, setAlertInfo] = useState<{ showAlert: boolean; message: string | null }>({ showAlert: false, message: null });
  const open = Boolean(anchorEl);

  const resolveSelected = (): ConfigKeyPair | undefined => {
    if (isEmpty(value)) {
      return undefined;
    }
    return keyPairs.find(k => k.id === value);
  };

  const refresh = async (): Promise<void> => {
    try {
      await configKeyPairService.reloadAll();
    } catch (err) {
      setAlertInfo({ showAlert: true, message: tf('home.common.loadFailedExclaim', { error: resolveErrorText(err) }) });
    }
  };

  const handleOpen = (e: MouseEvent<HTMLElement>): void => {
    setAnchorEl(e.currentTarget);
    void refresh();
  };

  const handleClose = (): void => {
    setAnchorEl(null);
    setSearchText(null);
  };

  const resolveSearchInputValue = (): string => {
    if (isNotEmpty(searchText)) {
      return searchText;
    }
    return '';
  };

  const handleSearchInputChange = (raw: string): void => {
    if (isEmpty(raw)) {
      setSearchText(null);
      return;
    }
    setSearchText(raw);
  };

  const resolveFilteredKeyPairs = (): ConfigKeyPair[] => {
    if (!isNotEmpty(searchText)) {
      return keyPairs;
    }
    const keyword = searchText.toLowerCase();
    return keyPairs.filter(k => k.name?.toLowerCase().includes(keyword) ?? false);
  };

  const handlePick = (keyPairId: string): void => {
    onChange(keyPairId);
    handleClose();
  };

  const handleCreateRequest = (): void => {
    setDraft({ keyPair: createEmptyConfigKeyPair(), isNew: true });
    handleClose();
  };

  const handleEditRequest = (kp: ConfigKeyPair, e: MouseEvent<Element>): void => {
    e.stopPropagation();
    setDraft({ keyPair: kp, isNew: false });
  };

  const handleDialogSave = (saved: ConfigKeyPair): void => {
    const wasNew = draft?.isNew === true;
    setDraft(null);
    void refresh();
    if (wasNew && saved.id) {
      onChange(saved.id);
    }
  };

  const handleDialogClose = (): void => {
    setDraft(null);
  };

  const resolveDisplayText = (): string => {
    const selected = resolveSelected();
    if (selected) {
      return selected.name ?? t('home.devices.keyPairUnnamed');
    }
    return t('home.devices.keyPairSelect');
  };

  const resolveValueColor = (): string => {
    if (resolveSelected()) {
      return 'var(--tm-text-primary)';
    }
    return 'var(--tm-text-tertiary)';
  };

  useEffect(() => {
    void refresh();
    const off = configKeyPairService.addKeyPairListChangeListener(list => {
      setKeyPairs(list);
    });
    return () => {
      off();
    };
  }, [configKeyPairService]);

  useEffect(() => {
    if (!open) {
      return;
    }
    const timer = window.setTimeout(() => {
      searchInputRef.current?.focus({ preventScroll: true });
    }, 32);
    return () => {
      window.clearTimeout(timer);
    };
  }, [open]);

  const filtered = resolveFilteredKeyPairs();

  const renderMenuBody = (mobile: boolean) => (
    <Box>
      <Box
        className={mobile ? 'tm-action-sheet-search' : undefined}
        sx={mobile ? undefined : { display: 'flex', alignItems: 'center', gap: 0.75, px: 1, py: 0.5, mx: 0.5, mt: 0.25, mb: 0.5, border: '1px solid var(--tm-border-subtle)', borderRadius: '6px', bgcolor: 'transparent', '&:focus-within': { borderColor: 'var(--tm-border-accent)' } }}
        onKeyDown={e => {
          e.stopPropagation();
          if (e.key === 'Escape') {
            handleClose();
          }
        }}
      >
        <Search sx={{ fontSize: mobile ? 18 : 14, color: 'var(--tm-text-tertiary)', flexShrink: 0 }} />
        <InputBase
          inputRef={searchInputRef}
          value={resolveSearchInputValue()}
          onChange={e => handleSearchInputChange(e.target.value)}
          placeholder={t('home.devices.keyPairSearch')}
          sx={{ flex: 1, fontSize: mobile ? 16 : 13, color: 'var(--tm-text-primary)', '& input::placeholder': { color: 'var(--tm-text-tertiary)', opacity: 1 } }}
        />
      </Box>
      <MenuItem className={mobile ? (isEmpty(value) ? 'tm-action-sheet-item is-selected' : 'tm-action-sheet-item') : isEmpty(value) ? 'tm-menu-item is-selected' : 'tm-menu-item'} onClick={() => handlePick('')}>
        <Box className="tm-menu-item-icon">{isEmpty(value) ? <Check /> : null}</Box>
        <Box className="tm-menu-item-text">{t('home.devices.keyPairNone')}</Box>
      </MenuItem>
      {filtered.length > 0 ? <Box className="tm-menu-divider" role="separator" /> : null}
      <Box className={mobile ? undefined : 'tm-menu-item-scroll'} sx={mobile ? undefined : { maxHeight: 240, overflowY: 'auto' }}>
        {filtered.map(k => (
          <MenuItem
            key={k.id}
            className={mobile ? (isNotEmpty(value) && k.id === value ? 'tm-action-sheet-item is-selected' : 'tm-action-sheet-item') : isNotEmpty(value) && k.id === value ? 'tm-menu-item is-selected' : 'tm-menu-item'}
            onClick={() => handlePick(k.id ?? '')}
            sx={{ '& .tm-key-edit-icon': { opacity: mobile ? 1 : 0, transition: 'opacity 120ms' }, '&:hover .tm-key-edit-icon': { opacity: 1 } }}
          >
            <Box className="tm-menu-item-icon">{isNotEmpty(value) && k.id === value ? <Check /> : null}</Box>
            <Box className="tm-keypair-name">{k.name}</Box>
            <DriveFileRenameOutline className="tm-key-edit-icon" onClick={e => handleEditRequest(k, e)} sx={{ fontSize: 15, flexShrink: 0, color: 'var(--tm-text-tertiary)', cursor: 'pointer', '&:hover': { color: 'var(--tm-text-primary)' } }} />
          </MenuItem>
        ))}
      </Box>
      <Box className="tm-menu-divider" role="separator" />
      <MenuItem className={mobile ? 'tm-action-sheet-item' : 'tm-menu-item'} onClick={handleCreateRequest}>
        <Box className="tm-menu-item-icon">
          <Add />
        </Box>
        <Box className="tm-menu-item-text">{t('home.devices.keyPairNew')}</Box>
      </MenuItem>
    </Box>
  );

  return (
    <Box>
      <Box
        className="tm-edit-field"
        sx={{ display: 'flex', alignItems: 'center', minHeight: '38px', border: '1px solid var(--tm-border-subtle)', borderRadius: '7px', bgcolor: 'transparent', transition: 'border-color 120ms ease, background-color 120ms ease', '&:hover': { bgcolor: 'var(--tm-bg-hover)' }, '&:focus-within': { bgcolor: 'var(--tm-bg-hover)', borderColor: 'var(--tm-border-accent)' }}}
      >
        <Typography className="tm-edit-field-label" sx={{ fontSize: 13, color: 'var(--tm-text-tertiary)', pl: 1, width: 64, flexShrink: 0, whiteSpace: 'nowrap', userSelect: 'none' }}>
          {t('home.devices.keyPairLabel')}
        </Typography>
        <ButtonBase onClick={handleOpen} disableRipple sx={{ flex: 1, minWidth: 0, justifyContent: 'flex-start', px: 1, py: 0.5, fontSize: 13, color: resolveValueColor(), textAlign: 'left', outline: 'none', '&:focus': { outline: 'none' }, '&.Mui-focusVisible': { outline: 'none' } }}>
          <Box sx={{ width: '100%', whiteSpace: 'nowrap', overflow: 'clip', textOverflow: 'ellipsis', textAlign: 'left' }}>{resolveDisplayText()}</Box>
        </ButtonBase>
        <Box sx={{ width: 26, height: 26, display: 'flex', alignItems: 'center', justifyContent: 'center', color: 'var(--tm-text-tertiary)', flexShrink: 0, mr: 0.75 }}>
          <VpnKey sx={{ fontSize: 14 }} />
        </Box>
      </Box>
      {mobileViewport ? (
        <ActionSheet open={open} onClose={handleClose}>
          <MenuList disablePadding className="tm-action-sheet-list">
            {renderMenuBody(true)}
          </MenuList>
        </ActionSheet>
      ) : (
        <Menu
          open={open}
          anchorEl={anchorEl}
          onClose={handleClose}
          anchorOrigin={{ vertical: 'bottom', horizontal: 'center' }}
          transformOrigin={{ vertical: 'top', horizontal: 'center' }}
          slotProps={{ paper: { className: 'tm-menu-paper', sx: { width: 230, mt: 0.5 } }, list: { className: 'tm-menu-list' } }}
        >
          {renderMenuBody(false)}
        </Menu>
      )}
      {draft ? <EditDialog open={true} configKeyPair={draft.keyPair} onSave={handleDialogSave} onClose={handleDialogClose} /> : null}
      <AlertDialog duration={1500} open={alertInfo.showAlert} handleClose={() => setAlertInfo({ showAlert: false, message: null })} message={alertInfo.message} type="error" />
    </Box>
  );
}
