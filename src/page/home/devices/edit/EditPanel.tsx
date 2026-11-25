import type { ReactNode } from 'react';
import { Footer } from './frame/Footer';
import { useEffect, useState } from 'react';
import { FtpSection } from './section/FtpSection';
import { RdpSection } from './section/RdpSection';
import { SshSection } from './section/SshSection';
import { PROTOCOL } from '@/common/const/Const.ts';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { SnmpSection } from './section/SnmpSection';
import type { Device } from '@/common/model/Device.ts';
import { ConfigFtp } from '@/common/model/ConfigFtp.ts';
import { ConfigRdp } from '@/common/model/ConfigRdp.ts';
import { ConfigSsh } from '@/common/model/ConfigSsh.ts';
import { TelnetSection } from './section/TelnetSection';
import { isNotEmpty } from '@/common/util/StringUtil.ts';
import { ConfigSnmp } from '@/common/model/ConfigSnmp.ts';
import { GeneralSection } from './section/GeneralSection';
import { ConfigTelnet } from '@/common/model/ConfigTelnet.ts';
import type { ReqDevice } from '@/common/model/req/ReqDevice.ts';
import { Box, ButtonBase, Dialog, Typography } from '@mui/material';
import { Header } from '@/page/common/components/dialog/Header.tsx';
import { useDeviceService } from '@/common/context/useAppHooks.tsx';
import { resolveErrorText } from '@/common/model/resp/ResultModel.ts';
import { useBackDismiss } from '@/page/common/hooks/useBackDismiss.ts';
import { usePlatformMobile } from '@/common/context/usePlatformMobile';
import type { ConfigTermBase } from '@/common/model/ConfigTermBase.ts';
import { AlertDialog } from '@/common/components/dialog/AlertDialog.tsx';
import { resolveSheetDialogProps } from '@/page/common/util/scopedDialogUtil.ts';
import { CellTower, DesktopWindows, FolderOpen, Lan, Terminal } from '@mui/icons-material';
import { verifyInfo, resolveDeviceError, resolveDeviceOsIcon, resolveDeviceOsIconChipClassName, resolveDeviceOsIconChipStyle } from '@/page/common/util/deviceUtil.ts';

type ProtocolTab = { protocol: number; label: string; icon: ReactNode };

const PROTOCOL_TABS: ProtocolTab[] = [
  { protocol: PROTOCOL.SSH, label: 'SSH', icon: <Terminal className="tm-device-edit-tab-icon" /> },
  { protocol: PROTOCOL.FTP, label: 'FTP', icon: <FolderOpen className="tm-device-edit-tab-icon" /> },
  { protocol: PROTOCOL.TELNET, label: 'Telnet', icon: <Lan className="tm-device-edit-tab-icon" /> },
  { protocol: PROTOCOL.RDP, label: 'RDP', icon: <DesktopWindows className="tm-device-edit-tab-icon" /> },
  { protocol: PROTOCOL.SNMP, label: 'SNMP', icon: <CellTower className="tm-device-edit-tab-icon" /> },
];

function hasTermCredential(base: ConfigTermBase | null | undefined): boolean {
  if (!base) {
    return false;
  }
  return isNotEmpty(base.username) || isNotEmpty(base.password);
}

function resolveInitialProtocol(req: ReqDevice, snmpVisible: boolean): number {
  if (hasTermCredential(req.config_ssh?.base) || isNotEmpty(req.config_ssh?.config_key_pair_id)) {
    return PROTOCOL.SSH;
  }
  if (hasTermCredential(req.config_telnet?.base)) {
    return PROTOCOL.TELNET;
  }
  if (hasTermCredential(req.config_rdp?.base)) {
    return PROTOCOL.RDP;
  }
  if (hasTermCredential(req.config_ftp?.base)) {
    return PROTOCOL.FTP;
  }
  if (snmpVisible && req.config_snmp?.enabled === true) {
    return PROTOCOL.SNMP;
  }
  return PROTOCOL.SSH;
}

export interface EditPanelProps {
  open: boolean;
  onClose: () => void;
  onCancel: () => void;
  reqDevice: ReqDevice;
  selectedAction: number;
  onSave?: (value: ReqDevice) => void;
  onChange?: (next: ReqDevice) => void;
  onConnect?: (value: ReqDevice) => void;
  onSelectedActionChange: (action: number) => void;
}

export function EditPanel(props: EditPanelProps) {
  const { t, tf } = useI18n();
  const mobileViewport = usePlatformMobile();
  const snmpVisible = !mobileViewport;
  const deviceService = useDeviceService();
  const [verify, setVerify] = useState<boolean>(false);
  const [devices, setDevices] = useState<ReqDevice[]>([]);
    const [value, setValue] = useState<ReqDevice>(props.reqDevice);
  const [selectedProtocol, setSelectedProtocol] = useState<number>(() => resolveInitialProtocol(props.reqDevice, snmpVisible));
  const { open, onChange, selectedAction, onSelectedActionChange, onSave, onConnect, onClose, onCancel } = props;
  const [alertInfo, setAlertInfo] = useState<{ showAlert: boolean; message: string | null }>({ showAlert: false, message: null });
  const error = resolveDeviceError(value, devices);
  const jumpOptions = devices.filter(item => item.device.id && item.device.id !== value.device.id).map(item => ({ id: item.device.id, label: item.device.name || item.device.host || item.device.id, os_type: item.device.os_type }));

  function resolveTabClass(protocol: number): string {
    if (selectedProtocol === protocol) {
      return 'tm-device-edit-tab is-on';
    }
    return 'tm-device-edit-tab';
  }

  const deviceVerify = (newDevice: ReqDevice): void => {
    setVerify(verifyInfo(newDevice, devices));
  };

  const holdReqDeviceChange = (newDevice: ReqDevice): void => {
    deviceVerify(newDevice);
    setValue(newDevice);
    onChange?.(newDevice);
  };

  const handleDeviceChange = (device: Device): void => {
    holdReqDeviceChange({ ...value, device });
  };

  const handleGroupChange = (groupId: string): void => {
    holdReqDeviceChange({ ...value, group_id: groupId });
  };

  const handleSshChange = (configSsh: ConfigSsh): void => {
    holdReqDeviceChange({ ...value, config_ssh: configSsh });
  };

  const handleTelnetChange = (configTelnet: ConfigTelnet): void => {
    holdReqDeviceChange({ ...value, config_telnet: configTelnet });
  };

  const handleRdpChange = (configRdp: ConfigRdp): void => {
    holdReqDeviceChange({ ...value, config_rdp: configRdp });
  };

  const handleFtpChange = (configFtp: ConfigFtp): void => {
    holdReqDeviceChange({ ...value, config_ftp: configFtp });
  };

  const handleSnmpChange = (configSnmp: ConfigSnmp): void => {
    holdReqDeviceChange({ ...value, config_snmp: configSnmp });
  };

  const handleSave = (): void => {
    if (!verify) {
      return;
    }
    deviceService
      .saveOrUpdateWithConfig(value)
      .then(saved => {
        if (!saved) {
          return;
        }
        setValue(saved);
        onSave?.(saved);
      })
      .catch(err => {
        setAlertInfo({ showAlert: true, message: tf('home.common.saveFailed', { error: resolveErrorText(err) }) });
      });
  };

  const handleConnect = (): void => {
    deviceService
      .saveOrUpdateWithConfig(value)
      .then(data => {
        if (!data) {
          return;
        }
        if (!onConnect) {
          return;
        }
        onConnect(data);
      })
      .catch(err => {
        setAlertInfo({ showAlert: true, message: tf('home.common.saveFailed', { error: resolveErrorText(err) }) });
      });
  };

  const handleAlertClose = (): void => {
    setAlertInfo({ showAlert: false, message: null });
  };

  useBackDismiss(open, onClose, 'device-edit');

  useEffect(() => {
    const offDevices = deviceService.addDeviceListChangeListener(list => {
      setDevices(list);
    });
    void deviceService.selectAllWithConfig();
    return offDevices;
  }, [deviceService]);

  useEffect(() => {
    setValue(props.reqDevice);
    setSelectedProtocol(resolveInitialProtocol(props.reqDevice, snmpVisible));
    deviceVerify(props.reqDevice);
  }, [snmpVisible, props.reqDevice]);

  useEffect(() => {
    deviceVerify(value);
  }, [devices]);

  return (
    <Dialog open={open} onClose={onClose} {...resolveSheetDialogProps({ className: 'tm-edit-dialog-paper tm-device-edit-dialog-paper' })}>
      <Box className="tm-edit-dialog-body">
        <Header
          title={t('home.devices.editTitle')}
          icon={
            <Box className={resolveDeviceOsIconChipClassName(value.device.os_type)} style={resolveDeviceOsIconChipStyle(value.device.os_type)}>
              {resolveDeviceOsIcon(value.device.os_type, 'tm-device-item-os-icon')}
            </Box>
          }
          onClose={onClose}
        />
        <Box className="tm-device-edit">
          <Box className="tm-device-edit-nav">
            {PROTOCOL_TABS.map(item =>
              item.protocol === PROTOCOL.SNMP && !snmpVisible ? null : (
                <ButtonBase key={item.protocol} disableRipple className={resolveTabClass(item.protocol)} onClick={() => setSelectedProtocol(item.protocol)}>
                  <Box className="tm-device-edit-tab-icon-wrap">{item.icon}</Box>
                  <Typography component="span" className="tm-device-edit-tab-label">
                    {item.label}
                  </Typography>
                </ButtonBase>
              ),
            )}
          </Box>
          <Box className="tm-device-edit-main">
            <GeneralSection respDevice={value} onChange={handleDeviceChange} onGroupChange={handleGroupChange} />
            {selectedProtocol === PROTOCOL.SSH ? <SshSection config={value.config_ssh} onChange={handleSshChange} jumpOptions={jumpOptions} /> : null}
            {selectedProtocol === PROTOCOL.TELNET ? <TelnetSection config={value.config_telnet} onChange={handleTelnetChange} /> : null}
            {selectedProtocol === PROTOCOL.RDP ? <RdpSection config={value.config_rdp} onChange={handleRdpChange} /> : null}
            {selectedProtocol === PROTOCOL.FTP ? <FtpSection config={value.config_ftp} onChange={handleFtpChange} /> : null}
            {snmpVisible && selectedProtocol === PROTOCOL.SNMP ? <SnmpSection config={value.config_snmp} onChange={handleSnmpChange} /> : null}
          </Box>
        </Box>
        <AlertDialog duration={1500} open={alertInfo.showAlert} handleClose={handleAlertClose} message={alertInfo.message} type="error" />
        <Footer disable={!verify} error={error} selectedAction={selectedAction} onSelectedActionChange={onSelectedActionChange} onCancel={onCancel} onSave={handleSave} onConnect={handleConnect} />
      </Box>
    </Dialog>
  );
}
