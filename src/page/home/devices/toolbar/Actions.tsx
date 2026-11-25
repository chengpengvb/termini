import { Icon } from '@iconify/react';
import type { ReactNode } from 'react';
import type { IconifyIcon } from '@iconify/types';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { useEffect, useRef, useState } from 'react';
import { ViewMode } from '../../components/ViewMode';
import { Dns, KeyboardArrowDown } from '@mui/icons-material';
import iconFileKey from '@iconify-icons/mdi/file-key-outline';
import iconTermius from '@iconify-icons/simple-icons/termius';
import { DEVICE_IMPORT_SOURCE } from '@/common/model/DeviceImport.ts';
import { usePlatformMobile } from '@/common/context/usePlatformMobile';
import { useSystemUtilService } from '@/common/context/useAppHooks.tsx';
import { Box, ButtonBase, Menu, MenuItem, MenuList } from '@mui/material';
import { ShellAction } from '@/page/home/devices/toolbar/ShellAction.tsx';
import { RespPlatformInfo } from '@/common/model/resp/RespPlatformInfo.ts';
import { SerialAction } from '@/page/home/devices/toolbar/SerialAction.tsx';
import { ActionSheet } from '@/page/common/components/dialog/ActionSheet.tsx';

export interface ActionsProps {
  viewMode: number;
  onAddHost: () => void;
  onChange: (mode: number) => void;
  onImport: (source: number) => void;
}

const IMPORT_SOURCES = [DEVICE_IMPORT_SOURCE.SSH_CONFIG, DEVICE_IMPORT_SOURCE.PUTTY, DEVICE_IMPORT_SOURCE.XSHELL, DEVICE_IMPORT_SOURCE.TERMIUS];

const ICON_PUTTY: IconifyIcon = {
  width: 24,
  height: 24,
  body: '<path fill="currentColor" d="M4 3h16a1 1 0 0 1 1 1v11a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1m1 2v9h14V5H5m4 13h6v1H9v-1m-3 1h12v2H6v-2Z"/>',
};

const ICON_XSHELL: IconifyIcon = {
  width: 24,
  height: 24,
  body: '<path fill="currentColor" d="M4 4h16a1 1 0 0 1 1 1v14a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V5a1 1 0 0 1 1-1m1 2v12h14V6H5m2.2 3.2 3.2 2.3-3.2 2.3 1 1.4 5-3.7-5-3.7-1 1.4M12 14.2h5v1.8h-5v-1.8Z"/>',
};

const ICON_TERMIUS: IconifyIcon = {
  width: 24,
  height: 24,
  body: `<g transform="translate(12 12) scale(0.88) translate(-12 -12)">${iconTermius.body}</g>`,
};

const IMPORT_ICONS: Record<number, ReactNode> = {
  [DEVICE_IMPORT_SOURCE.SSH_CONFIG]: <Icon icon={iconFileKey} inline={false} />,
  [DEVICE_IMPORT_SOURCE.PUTTY]: <Icon icon={ICON_PUTTY} inline={false} />,
  [DEVICE_IMPORT_SOURCE.XSHELL]: <Icon icon={ICON_XSHELL} inline={false} />,
  [DEVICE_IMPORT_SOURCE.TERMIUS]: <Icon icon={ICON_TERMIUS} inline={false} />,
};

export function Actions(props: ActionsProps) {
  const { t } = useI18n();
  const mobileViewport = usePlatformMobile();
  const systemUtilService = useSystemUtilService();
  const anchorRef = useRef<HTMLDivElement | null>(null);
  const { viewMode, onChange, onAddHost, onImport } = props;
  const [importOpen, setImportOpen] = useState<boolean>(false);
  const [platform, setPlatform] = useState<RespPlatformInfo | null>(null);

  const handleImportToggle = (): void => {
    setImportOpen(prev => !prev);
  };

  const handleImportClose = (): void => {
    setImportOpen(false);
  };

  const handleImportPick = (source: number): void => {
    setImportOpen(false);
    onImport(source);
  };

  const renderImportItems = (mobile: boolean) => {
    return IMPORT_SOURCES.map(source => (
      <MenuItem key={source} dense={!mobile} className={mobile ? 'tm-action-sheet-item' : 'tm-menu-item'} onClick={() => handleImportPick(source)}>
        <Box component="span" className="tm-menu-item-icon tm-home-import-menu-icon">
          {IMPORT_ICONS[source]}
        </Box>
        <Box component="span" className="tm-menu-item-text">
          {t(`home.devices.importMenu.${source}`)}
        </Box>
      </MenuItem>
    ));
  };

  useEffect(() => {
    systemUtilService.getPlatformInfo().then(info => {
      setPlatform(info);
    });
  }, [systemUtilService]);

  const importEnabled = platform === null || !platform.mobile;

  return (
    <Box className="tm-home-actions-row">
      <Box ref={anchorRef} className={importOpen ? 'tm-shell-action is-open' : 'tm-shell-action'}>
        <ButtonBase onClick={onAddHost} data-tm-add-host="true" className={importEnabled ? 'tm-action-btn tm-action-btn-primary tm-action-left-btn tm-shell-action-main' : 'tm-action-btn tm-action-btn-primary tm-shell-action-main is-solo'}>
          <Dns />
          <Box component="span" className="tm-action-btn-label tm-shell-action-label">
            {t('home.devices.addHost')}
          </Box>
        </ButtonBase>
        {importEnabled ? (
          <ButtonBase onClick={handleImportToggle} className="tm-action-btn tm-action-btn-primary tm-action-right-btn tm-shell-action-more" aria-label={t('home.devices.importHost')}>
            <KeyboardArrowDown />
          </ButtonBase>
        ) : null}
      </Box>
      {platform?.has_local_shell ? <ShellAction /> : null}
      {platform?.has_serial ? <SerialAction /> : null}
      <Box className="tm-home-actions-spacer" />
      <ViewMode viewMode={viewMode} onChange={onChange} />
      {!importEnabled ? null : mobileViewport ? (
        <ActionSheet open={importOpen} onClose={handleImportClose}>
          <MenuList disablePadding className="tm-action-sheet-list">
            {renderImportItems(true)}
          </MenuList>
        </ActionSheet>
      ) : (
        <Menu
          open={importOpen}
          onClose={handleImportClose}
          anchorEl={anchorRef.current}
          anchorOrigin={{ vertical: 'bottom', horizontal: 'left' }}
          transformOrigin={{ vertical: 'top', horizontal: 'left' }}
          slotProps={{ paper: { className: 'tm-menu-paper', sx: { zIndex: 1600, mt: '-4px' } }, list: { className: 'tm-menu-list' } }}
        >
          {renderImportItems(false)}
        </Menu>
      )}
    </Box>
  );
}
