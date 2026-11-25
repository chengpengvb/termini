import { useI18n } from '@/common/i18n/useI18n.ts';
import { useEffect, useRef, useState } from 'react';
import { ShellDescr } from '@/common/model/ShellDescr.ts';
import { ShellPage } from '@/page/protocol/shell/ShellPage.tsx';
import { usePlatformMobile } from '@/common/context/usePlatformMobile';
import { ActionSheet } from '@/page/common/components/dialog/ActionSheet.tsx';
import { Box, ButtonBase, Menu, MenuItem, MenuList, Typography } from '@mui/material';
import { ArrowForward, Check, KeyboardArrowDown, Terminal } from '@mui/icons-material';
import { useShellService, useTabManagerService } from '@/common/context/useAppHooks.tsx';
import { resolveShellIcon, resolveShellKind, resolveShellLabel, SHELL_KIND } from '@/page/common/util/shellUtil.ts';

export function ShellAction() {
  const { t } = useI18n();
  const shellService = useShellService();
  const mobileViewport = usePlatformMobile();
  const tabManagerService = useTabManagerService();
  const [open, setOpen] = useState<boolean>(false);
  const [list, setList] = useState<ShellDescr[]>([]);
  const selectedRef = useRef<ShellDescr | null>(null);
  const anchorRef = useRef<HTMLDivElement | null>(null);
  const [selected, setSelected] = useState<ShellDescr | null>(null);

  const resolveDisplay = (item: ShellDescr | null) => {
    if (!item) {
      return {
        icon: resolveShellIcon(SHELL_KIND.UNKNOWN),
        label: resolveShellLabel(SHELL_KIND.UNKNOWN),
      };
    }
    const kind = resolveShellKind(item.shell_id);
    return {
      icon: resolveShellIcon(kind),
      label: resolveShellLabel(kind),
    };
  };

  const handleLaunch = () => {
    const current = selectedRef.current;
    if (!current) {
      return;
    }
    const kind = resolveDisplay(current);
    tabManagerService.openTab({
      panes: [
        {
          tabBar: {
            canClose: true,
            icon: kind.icon,
            title: kind.label,
          },
          content: <ShellPage shellDescr={current} />,
        },
      ],
    });
  };

  const handleToggle = () => {
    setOpen(prev => !prev);
  };

  const handleClose = () => {
    setOpen(false);
  };

  const handlePick = (item: ShellDescr) => {
    selectedRef.current = item;
    setSelected(item);
    setOpen(false);
    handleLaunch();
  };

  useEffect(() => {
    shellService.discover().then(items => {
      setList(items);
      if (items.length == 0) {
        return;
      }
      selectedRef.current = items[0];
      setSelected(items[0]);
    });
  }, [shellService]);

  const display = resolveDisplay(selected);

  const renderItems = (mobile: boolean) => {
    return list.map(item => {
      const itemDisplay = resolveDisplay(item);
      const selectedItem = selected?.shell_id === item.shell_id;
      return (
        <MenuItem key={item.shell_id} dense={!mobile} className={mobile ? (selectedItem ? 'tm-action-sheet-item is-selected' : 'tm-action-sheet-item') : 'tm-menu-item'} onClick={() => handlePick(item)}>
          <Box component="span" className="tm-menu-item-icon">
            {itemDisplay.icon}
          </Box>
          <Box component="span" className="tm-menu-item-text">
            {itemDisplay.label}
          </Box>
          {mobile && selectedItem ? <Check className="tm-action-sheet-check" /> : null}
        </MenuItem>
      );
    });
  };

  const renderMobileCard = () => {
    const multiple = list.length > 1;
    return (
      <Box className="tm-home-local-card">
        <ButtonBase disableRipple onClick={handleLaunch} className="tm-home-local-card-main">
          <Box className="tm-home-local-card-icon">
            <Terminal />
          </Box>
          <Box className="tm-home-local-card-info">
            <Typography className="tm-home-local-card-title tm-ellipsis">{t('home.devices.localTerminal')}</Typography>
            <Typography className="tm-home-local-card-hint tm-ellipsis">{t('home.devices.localTerminalHint')}</Typography>
          </Box>
        </ButtonBase>
        <ButtonBase disableRipple onClick={multiple ? handleToggle : handleLaunch} className="tm-home-local-card-open tm-btn-tinted is-success" aria-label={multiple ? t('home.devices.shellMore') : t('home.devices.open')}>
          <Box component="span" className="tm-home-local-card-open-label">
            {t('home.devices.open')}
          </Box>
          {multiple ? <KeyboardArrowDown /> : <ArrowForward />}
        </ButtonBase>
      </Box>
    );
  };

  const renderPill = () => {
    return (
      <Box ref={anchorRef} className={open ? 'tm-shell-action is-open' : 'tm-shell-action'}>
        <ButtonBase onClick={handleLaunch} className="tm-action-btn tm-action-left-btn tm-shell-action-main">
          {display.icon}
          <Box component="span" className="tm-action-btn-label tm-shell-action-label">
            {display.label}
          </Box>
        </ButtonBase>
        <ButtonBase onClick={handleToggle} className="tm-action-btn tm-action-right-btn tm-shell-action-more" aria-label={t('home.devices.shellMore')}>
          <KeyboardArrowDown />
        </ButtonBase>
      </Box>
    );
  };

  return (
    <>
      {mobileViewport ? renderMobileCard() : renderPill()}
      {mobileViewport ? (
        <ActionSheet open={open} title={t('home.devices.shellSelectTerminal')} onClose={handleClose}>
          <MenuList disablePadding className="tm-action-sheet-list">
            {renderItems(true)}
          </MenuList>
        </ActionSheet>
      ) : (
        <Menu
          open={open}
          onClose={handleClose}
          anchorEl={anchorRef.current}
          anchorOrigin={{ vertical: 'bottom', horizontal: 'left' }}
          transformOrigin={{ vertical: 'top', horizontal: 'left' }}
          slotProps={{ paper: { className: 'tm-menu-paper', sx: { zIndex: 1600, mt: '-4px' } }, list: { className: 'tm-menu-list' } }}
        >
          {renderItems(false)}
        </Menu>
      )}
    </>
  );
}
