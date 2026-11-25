import type { ReactNode } from 'react';
import { PROTOCOL } from '@/common/const/Const.ts';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { isNil } from '@/common/util/StringUtil.ts';
import { Box, ButtonBase, Typography } from '@mui/material';
import type { ReqDevice } from '@/common/model/req/ReqDevice.ts';
import { DesktopWindows, Folder, Lan, Terminal } from '@mui/icons-material';
import { resolveSelectableProtocols } from '@/page/common/util/protocolOpenUtil.ts';
import { FTP_DEFAULT_PORT, RDP_DEFAULT_PORT, SSH_DEFAULT_PORT, TELNET_DEFAULT_PORT } from '@/common/const/DeviceConst.ts';

export interface ProtocolCardProps {
  reqDevice: ReqDevice;
  selectedProtocols: number[];
  onToggle: (protocol: number) => void;
  doConnect: (protocols: number[]) => void;
}

export function ProtocolCard(props: ProtocolCardProps): ReactNode {
  const { tf } = useI18n();
  const { reqDevice, selectedProtocols, onToggle, doConnect } = props;

  const host = reqDevice.device.host;
  const protocolOrder = resolveSelectableProtocols(reqDevice);
  if (protocolOrder.length === 0) {
    return null;
  }

  const resolveIcon = (protocol: number): ReactNode => {
    const ICON_MAP: Record<number, ReactNode> = {
      [PROTOCOL.SSH]: <Terminal sx={{ fontSize: 16 }} />,
      [PROTOCOL.TELNET]: <Lan sx={{ fontSize: 16 }} />,
      [PROTOCOL.RDP]: <DesktopWindows sx={{ fontSize: 16 }} />,
      [PROTOCOL.SFTP]: <Folder sx={{ fontSize: 16 }} />,
      [PROTOCOL.FTP]: <Folder sx={{ fontSize: 16 }} />,
    };
    return ICON_MAP[protocol];
  };

  const resolveLabel = (protocol: number): string => {
    const LABEL_MAP: Record<number, string> = {
      [PROTOCOL.SSH]: 'SSH',
      [PROTOCOL.RDP]: 'RDP',
      [PROTOCOL.SFTP]: 'SFTP',
      [PROTOCOL.TELNET]: 'Telnet',
      [PROTOCOL.FTP]: 'FTP',
    };
    return LABEL_MAP[protocol];
  };

  const resolvePortValue = (port: number | null | undefined, fallback: number): number => {
    if (isNil(port) || port === 0) {
      return fallback;
    }
    return port;
  };

  const resolveMeta = (protocol: number): string | null => {
    if (protocol === PROTOCOL.SSH) {
      if (!reqDevice.config_ssh || !reqDevice.config_ssh.base) {
        return null;
      }
      return tf('common.portWithValue', { port: resolvePortValue(reqDevice.config_ssh.base.port, SSH_DEFAULT_PORT) });
    }
    if (protocol === PROTOCOL.TELNET) {
      if (!reqDevice.config_telnet || !reqDevice.config_telnet.base) {
        return null;
      }
      return tf('common.portWithValue', { port: resolvePortValue(reqDevice.config_telnet.base.port, TELNET_DEFAULT_PORT) });
    }
    if (protocol === PROTOCOL.RDP) {
      if (!reqDevice.config_rdp || !reqDevice.config_rdp.base) {
        return null;
      }
      return tf('common.portWithValue', { port: resolvePortValue(reqDevice.config_rdp.base.port, RDP_DEFAULT_PORT) });
    }
    if (protocol === PROTOCOL.SFTP) {
      if (!reqDevice.config_ssh || !reqDevice.config_ssh.base) {
        return null;
      }
      return tf('common.portWithValue', { port: resolvePortValue(reqDevice.config_ssh.base.port, SSH_DEFAULT_PORT) });
    }
    if (protocol === PROTOCOL.FTP) {
      if (!reqDevice.config_ftp || !reqDevice.config_ftp.base) {
        return null;
      }
      return tf('common.portWithValue', { port: resolvePortValue(reqDevice.config_ftp.base.port, FTP_DEFAULT_PORT) });
    }
    return null;
  };

  const resolveDetail = (_protocol: number, label: string): string => {
    return `${label} ${host}`;
  };

  return (
    <Box className="tm-protocol-select-list">
      {protocolOrder.map(protocol => {
        const label = resolveLabel(protocol);
        const meta = resolveMeta(protocol);
        const selected = selectedProtocols.includes(protocol);
        return (
          <ButtonBase disableRipple key={protocol} onClick={() => onToggle(protocol)} onDoubleClick={() => doConnect([protocol])} className={selected ? 'tm-device-item tm-protocol-card is-selected' : 'tm-device-item tm-protocol-card'}>
            <Box className={selected ? 'tm-protocol-icon-tile tm-protocol-icon-tile--md is-active' : 'tm-protocol-icon-tile tm-protocol-icon-tile--md'}>{resolveIcon(protocol)}</Box>
            <Box className="tm-protocol-card-body">
              <Typography className="tm-protocol-card-label">{label}</Typography>
              <Box className="tm-protocol-card-sub">
                <Typography className="tm-protocol-card-detail">{resolveDetail(protocol, label)}</Typography>
                {meta ? <Typography className="tm-protocol-card-meta">{meta}</Typography> : null}
              </Box>
            </Box>
          </ButtonBase>
        );
      })}
    </Box>
  );
}
