import { useI18n } from '@/common/i18n/useI18n.ts';
import type { MouseEvent, ReactNode } from 'react';
import { useEffect, useRef, useState } from 'react';
import type { SelectChangeEvent } from '@mui/material';
import { ArrowDropDown, Check, Search } from '@mui/icons-material';
import { usePlatformMobile } from '@/common/context/usePlatformMobile';
import { isEmpty, isNotEmpty, isNotNil } from '@/common/util/StringUtil.ts';
import { ActionSheet } from '@/page/common/components/dialog/ActionSheet.tsx';
import { Box, InputBase, Menu, MenuItem, MenuList, Select, Typography } from '@mui/material';

export interface SelectOption {
  value: string;
  label: string;
  icon?: ReactNode;
  data?: Object;
}

export function SelectField(props: { label?: string; sheetTitle?: string; value: string | null | undefined; onChange: (value: string, data?: Object) => void; options: SelectOption[]; labelWidth?: number; placeholder?: string; menuUp?: boolean; searchable?: boolean; searchPlaceholder?: string }) {
  const { t } = useI18n();
  const mobileViewport = usePlatformMobile();
  const [searchText, setSearchText] = useState<string | null>(null);
  const [anchorEl, setAnchorEl] = useState<HTMLElement | null>(null);
  const searchInputRef = useRef<HTMLInputElement | null>(null);
  const { label, sheetTitle, value, onChange, options, labelWidth, placeholder, menuUp, searchable, searchPlaceholder } = props;
  const open = Boolean(anchorEl);

  const resolveLabelWidth = (): number => {
    if (isNotNil(labelWidth)) {
      return labelWidth;
    }
    return 64;
  };

  const renderOptionTrigger = (opt: SelectOption): ReactNode => {
    if (!isNotNil(opt.icon)) {
      return opt.label;
    }
    return (
      <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.75, minWidth: 0, width: '100%' }}>
        <Box sx={{ display: 'flex', flexShrink: 0, alignItems: 'center' }}>{opt.icon}</Box>
        <Box sx={{ minWidth: 0, overflow: 'clip', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{opt.label}</Box>
      </Box>
    );
  };

  const renderMenuOption = (opt: SelectOption): ReactNode => {
    return (
      <>
        {isNotNil(opt.icon) ? opt.icon : null}
        <Box component="span" className="tm-menu-item-text">
          {opt.label}
        </Box>
      </>
    );
  };

  const resolveSelectedLabel = (): ReactNode => {
    const found = options.find(o => o.value === (value ?? ''));
    if (isNotNil(found)) {
      return renderOptionTrigger(found);
    }
    if (isNotEmpty(placeholder)) {
      return (
        <Box component="span" sx={{ color: 'var(--tm-text-tertiary)', opacity: 0.7 }}>
          {placeholder}
        </Box>
      );
    }
    return value ?? '';
  };

  const resolveFilteredOptions = (): SelectOption[] => {
    if (!isNotEmpty(searchText)) {
      return options;
    }
    const keyword = searchText.toLowerCase();
    return options.filter(opt => opt.label.toLowerCase().includes(keyword) || opt.value.toLowerCase().includes(keyword));
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

  const handleOpen = (e: MouseEvent<HTMLElement>): void => {
    setAnchorEl(e.currentTarget);
  };

  const handleClose = (): void => {
    setAnchorEl(null);
    setSearchText(null);
  };

  const handlePick = (nextValue: string): void => {
    const found = options.find(o => o.value === nextValue);
    if (isNotNil(found) && isNotNil(found.data)) {
      onChange(nextValue, found.data);
    } else {
      onChange(nextValue);
    }
    handleClose();
  };

  const handleChange = (e: SelectChangeEvent): void => {
    const nextValue = e.target.value;
    const found = options.find(o => o.value === nextValue);
    if (isNotNil(found) && isNotNil(found.data)) {
      onChange(nextValue, found.data);
      return;
    }
    onChange(nextValue);
  };

  useEffect(() => {
    if (!open || searchable !== true) {
      return;
    }
    const timer = window.setTimeout(() => {
      searchInputRef.current?.focus({ preventScroll: true });
    }, 32);
    return () => {
      window.clearTimeout(timer);
    };
  }, [open, searchable]);

  const renderSelectedValue = (v: string): ReactNode => {
    const found = options.find(o => o.value === v);
    if (isNotNil(found)) {
      return renderOptionTrigger(found);
    }
    if (isNotEmpty(placeholder)) {
      return (
        <Box component="span" sx={{ color: 'var(--tm-text-tertiary)', opacity: 0.7 }}>
          {placeholder}
        </Box>
      );
    }
    return v;
  };

  const renderOptionList = (mobile: boolean) => {
    const filtered = searchable === true ? resolveFilteredOptions() : options;
    return (
      <Box>
        {searchable === true ? (
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
              placeholder={isNotEmpty(searchPlaceholder) ? searchPlaceholder : t('common.searchEllipsis')}
              sx={{ flex: 1, fontSize: mobile ? 16 : 13, color: 'var(--tm-text-primary)', '& input::placeholder': { color: 'var(--tm-text-tertiary)', opacity: 1 } }}
            />
          </Box>
        ) : null}
        {mobile ? (
          <MenuList disablePadding className="tm-action-sheet-list">
            {filtered.map(opt => (
              <MenuItem key={opt.value} className={isNotEmpty(value) && opt.value === value ? 'tm-action-sheet-item is-selected' : 'tm-action-sheet-item'} onClick={() => handlePick(opt.value)}>
                {renderMenuOption(opt)}
                {isNotEmpty(value) && opt.value === value ? <Check className="tm-action-sheet-check" /> : null}
              </MenuItem>
            ))}
            {filtered.length === 0 ? <Box className="tm-action-sheet-empty">{t('common.noMatch')}</Box> : null}
          </MenuList>
        ) : (
          <Box sx={{ maxHeight: 280, overflowY: 'auto' }}>
            {filtered.map(opt => (
              <MenuItem key={opt.value} className={isNotEmpty(value) && opt.value === value ? 'tm-menu-item is-selected' : 'tm-menu-item'} onClick={() => handlePick(opt.value)} sx={{ minWidth: 0, maxWidth: '100%' }}>
                {renderMenuOption(opt)}
              </MenuItem>
            ))}
            {filtered.length === 0 ? <Box sx={{ px: 1.5, py: 1, fontSize: 12, color: 'var(--tm-text-tertiary)' }}>{t('common.noMatch')}</Box> : null}
          </Box>
        )}
      </Box>
    );
  };

  if (searchable === true || mobileViewport) {
    return (
      <Box>
        <Box
          className="tm-edit-field"
          sx={{ display: 'flex', alignItems: 'center', width: '100%', maxWidth: '100%', minWidth: 0, flexShrink: 0, boxSizing: 'border-box', minHeight: '38px', border: '1px solid var(--tm-border-subtle)', borderRadius: '7px', bgcolor: 'transparent', transition: 'border-color 120ms ease, background-color 120ms ease', '&:hover': { bgcolor: 'var(--tm-bg-hover)' }, '&:focus-within': { bgcolor: 'var(--tm-bg-hover)', borderColor: 'var(--tm-border-accent)' }}}
          onClick={handleOpen}
        >
          {isNotNil(label) ? (
            <Typography className="tm-edit-field-label" sx={{ fontSize: 13, color: 'var(--tm-text-tertiary)', pl: 1, width: resolveLabelWidth(), flexShrink: 0, whiteSpace: 'nowrap', userSelect: 'none' }}>
              {label}
            </Typography>
          ) : null}
          <Box className="tm-edit-field-value" sx={{ flex: 1, minWidth: 0, px: 1, py: 0.5, fontSize: 13, color: 'var(--tm-text-primary)', display: 'flex', alignItems: 'center', overflow: 'clip', textOverflow: 'ellipsis', whiteSpace: 'nowrap', cursor: 'pointer' }}>
            {resolveSelectedLabel()}
          </Box>
          <ArrowDropDown className={open ? 'tm-edit-field-caret is-open' : 'tm-edit-field-caret'} />
        </Box>
        {mobileViewport ? (
          <ActionSheet open={open} title={sheetTitle ?? label} onClose={handleClose}>
            {renderOptionList(true)}
          </ActionSheet>
        ) : (
          <Menu
            open={open}
            anchorEl={anchorEl}
            onClose={handleClose}
            anchorOrigin={{ vertical: menuUp === true ? 'top' : 'bottom', horizontal: 'center' }}
            transformOrigin={{ vertical: menuUp === true ? 'bottom' : 'top', horizontal: 'center' }}
            slotProps={{ paper: { className: 'tm-menu-paper', sx: { width: anchorEl ? Math.max(anchorEl.clientWidth, 220) : 220, mt: 0.5, maxWidth: 360 } }, list: { className: 'tm-menu-list', sx: { py: 0.5 } } }}
          >
            {renderOptionList(false)}
          </Menu>
        )}
      </Box>
    );
  }

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
      <Select
        className="tm-edit-field-input"
        value={value ?? ''}
        displayEmpty
        disableUnderline
        variant="standard"
        onChange={handleChange}
        renderValue={renderSelectedValue}
        sx={{ flex: 1, minWidth: 0, px: 1, fontSize: 13, color: 'var(--tm-text-primary)', '& .MuiSelect-select': { display: 'flex', alignItems: 'center', py: 0.5, px: 0, pr: 3, minWidth: 0, overflow: 'clip', '&:focus': { bgcolor: 'transparent' }}, '& .MuiSvgIcon-root': { color: 'var(--tm-text-tertiary)' }, '& .MuiSelect-icon': { right: '6px' }}}
        MenuProps={{
          anchorOrigin: { vertical: menuUp === true ? 'top' : 'bottom', horizontal: 'center' },
          transformOrigin: { vertical: menuUp === true ? 'bottom' : 'top', horizontal: 'center' },
          slotProps: { paper: { className: 'tm-menu-paper', sx: { minWidth: 220, maxHeight: 280, maxWidth: 360 } }, list: { className: 'tm-menu-list' } },
        }}
      >
        {options.map(opt => (
          <MenuItem key={opt.value} value={opt.value} className="tm-menu-item" sx={{ minWidth: 0, maxWidth: '100%' }}>
            {renderMenuOption(opt)}
          </MenuItem>
        ))}
      </Select>
    </Box>
  );
}
