import { TopBar } from './toolbar/TopBar';
import { Content } from './content/Content';
import { useEffect, useState } from 'react';
import { EditPanel } from './edit/EditPanel';
import { Box, Typography } from '@mui/material';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { VIEW_MODE } from '@/common/const/Const.ts';
import { ImportDialog } from './import/ImportDialog.tsx';
import { DEVICE_IMPORT_SOURCE } from '@/common/model/DeviceImport.ts';
import { resolveDeviceTabIcon } from '@/page/common/util/deviceUtil.ts';
import { ShellAction } from '@/page/home/devices/toolbar/ShellAction.tsx';
import { RespPlatformInfo } from '@/common/model/resp/RespPlatformInfo.ts';
import { ReqDevice, createEmptyReqDevice } from '@/common/model/req/ReqDevice.ts';
import { SelectProtocol } from '@/page/protocol/common/select/SelectProtocol.tsx';
import { usePlatformMobile, usePlatformNativeMobile } from '@/common/context/usePlatformMobile';
import { useDeviceEditService, useDeviceService, useSystemUtilService, useTabManagerService } from '@/common/context/useAppHooks.tsx';

export function DevicePage(props: { active: boolean }) {
  const { active } = props;
  const { t } = useI18n();
  const mobileViewport = usePlatformMobile();
  const nativeMobile = usePlatformNativeMobile();
  const deviceService = useDeviceService();
  const deviceEditService = useDeviceEditService();
  const tabManagerService = useTabManagerService();
  const systemUtilService = useSystemUtilService();
  const [viewMode, setViewMode] = useState<number>(VIEW_MODE.GRID);
  const [searchText, setSearchText] = useState<string | null>(null);
  const [drawerVisible, setDrawerVisible] = useState<boolean>(false);
  const [importVisible, setImportVisible] = useState<boolean>(false);
  const [platform, setPlatform] = useState<RespPlatformInfo | null>(null);
  const [reqDevice, setReqDevice] = useState<ReqDevice>(createEmptyReqDevice());
  const [importSource, setImportSource] = useState<number>(DEVICE_IMPORT_SOURCE.SSH_CONFIG);
  const [selectedAction, setSelectedAction] = useState<number>(() => deviceEditService.getSelectedAction());

  const resolveWithDraft = (value: ReqDevice): ReqDevice => {
    const draft = deviceEditService.getDraft(value.device.id);
    if (draft === null) {
      return value;
    }
    return draft;
  };

  const handleOpenDevice = (value: ReqDevice): void => {
    setReqDevice(resolveWithDraft(value));
  };

  const handleAddHost = (): void => {
    if (drawerVisible) {
      setDrawerVisible(false);
      return;
    }
    setReqDevice(resolveWithDraft(createEmptyReqDevice()));
    setDrawerVisible(true);
  };

  const handleClose = (): void => {
    setDrawerVisible(false);
  };

  const handleCancel = (): void => {
    deviceEditService.clearDraft(reqDevice.device.id);
    setDrawerVisible(false);
  };

  const handleDraftChange = (next: ReqDevice): void => {
    deviceEditService.setDraft(next);
  };

  const handleImportOpen = (source: number): void => {
    setImportSource(source);
    setImportVisible(true);
  };

  const handleImportClose = (): void => {
    setImportVisible(false);
  };

  const handleImported = (): void => {
    void deviceService.selectAllWithConfig();
  };

  const handleSelectedActionChange = (action: number): void => {
    deviceEditService.setSelectedAction(action);
  };

  const handleSave = (_value: ReqDevice): void => {
    deviceEditService.clearDraft(reqDevice.device.id);
    setDrawerVisible(false);
  };

  const resolveViewMode = (): number => {
    if (mobileViewport) {
      return VIEW_MODE.LIST;
    }
    return viewMode;
  };

  const handleConnect = (value: ReqDevice): void => {
    deviceEditService.clearDraft(reqDevice.device.id);
    tabManagerService.openTab({
      panes: [
        {
          tabBar: {
            canClose: true,
            title: value.device.name,
            icon: resolveDeviceTabIcon(value.device.os_type),
          },
          content: <SelectProtocol reqDevice={value} />,
        },
      ],
    });
    setDrawerVisible(false);
  };

  const renderContent = () => {
    return <Content active={active} viewMode={resolveViewMode()} searchText={searchText} onBlankClick={handleClose} switchVisible={handleAddHost} setReqDevice={handleOpenDevice} setDrawerVisible={setDrawerVisible} />;
  };

  const renderMobileBody = () => {
    return (
      <Box className="tm-home-mobile-scroll">
        {!nativeMobile && platform?.has_local_shell ? (
          <Box className="tm-home-section">
            <Typography className="tm-home-section-label">{t('home.devices.localSection')}</Typography>
            <ShellAction />
          </Box>
        ) : null}
        <Box className="tm-home-section">
          {renderContent()}
        </Box>
      </Box>
    );
  };

  useEffect(() => {
    const offAction = deviceEditService.addSelectedActionListener(setSelectedAction);
    return () => {
      offAction();
    };
  }, [deviceEditService]);

  useEffect(() => {
    if (!mobileViewport || nativeMobile) {
      return;
    }
    systemUtilService.getPlatformInfo().then(setPlatform);
  }, [systemUtilService, mobileViewport, nativeMobile]);

  return (
    <Box className={mobileViewport ? 'tm-home-device-root is-mobile' : 'tm-home-device-root'}>
      <Box className="tm-home-page">
        <TopBar viewMode={viewMode} onViewModeChange={setViewMode} onAddHost={handleAddHost} onImport={handleImportOpen} onQueryChange={setSearchText} onConfirm={handleConnect} />
        {mobileViewport ? renderMobileBody() : renderContent()}
      </Box>
      <EditPanel open={drawerVisible} reqDevice={reqDevice} selectedAction={selectedAction} onSelectedActionChange={handleSelectedActionChange} onChange={handleDraftChange} onSave={handleSave} onConnect={handleConnect} onClose={handleClose} onCancel={handleCancel} />
      <ImportDialog open={importVisible} source={importSource} onClose={handleImportClose} onImported={handleImported} />
    </Box>
  );
}
