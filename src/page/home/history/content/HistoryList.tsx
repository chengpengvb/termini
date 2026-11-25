import { useState } from 'react';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { isEmpty } from '@/common/util/StringUtil.ts';
import { usePaneContext } from '@/common/components/tab';
import { Box, ButtonBase, Typography } from '@mui/material';
import { Item } from '@/page/home/history/content/Item.tsx';
import type { ReqDevice } from '@/common/model/req/ReqDevice.ts';
import type { HistoryLogin } from '@/common/model/dto/HistoryLogin.ts';
import { DeleteOutline, ExpandLess, ExpandMore } from '@mui/icons-material';
import { resolveDeviceOsIconBadged } from '@/page/common/util/deviceUtil.ts';
import { openDeviceProtocol, resolveProtocolLabel } from '@/page/common/util/protocolOpenUtil.ts';
import { useDeviceService, useHistoryLoginService, useTabManagerService } from '@/common/context/useAppHooks.tsx';

export interface HistoryListProps {
  items: HistoryLogin[];
  deviceMap: Map<string, ReqDevice>;
}

const COLLAPSED_COUNT = 5;

export function HistoryList(props: HistoryListProps) {
  const { t, tf } = useI18n();
  const { items, deviceMap } = props;
  const { paneId } = usePaneContext();
  const deviceService = useDeviceService();
  const tabManagerService = useTabManagerService();
  const historyLoginService = useHistoryLoginService();
  const [expanded, setExpanded] = useState<boolean>(false);

  const handleOpen = async (reqDevice: ReqDevice, protocol: number): Promise<void> => {
    const opened = await openDeviceProtocol({
      protocol,
      reqDevice,
      tabManagerService,
      resolveLocalDevice: () => deviceService.localDevice(),
    });
    if (!opened) {
      return;
    }
    historyLoginService.add(reqDevice.device.id, protocol);
    tabManagerService.closePane(paneId);
  };

  const handleClear = (): void => {
    historyLoginService.clear();
    setExpanded(false);
  };

  const handleToggleExpand = (): void => {
    setExpanded(prev => !prev);
  };

  const rows = items.flatMap(item => {
    const reqDevice = deviceMap.get(item.device_id);
    if (!reqDevice || isEmpty(reqDevice.device.name)) {
      return [];
    }
    return [{ item, reqDevice }];
  });

  const canCollapse = rows.length > COLLAPSED_COUNT;
  const visibleRows = !canCollapse || expanded ? rows : rows.slice(0, COLLAPSED_COUNT);

  return (
    <Box className="tm-history-section">
      <Box className="tm-history-section-head">
        <Typography className="tm-history-section-title">{t('home.history.recentTitle')}</Typography>
        <ButtonBase disableRipple className="tm-history-section-clear" onClick={handleClear} aria-label={t('home.history.clearLabel')}>
          <DeleteOutline className="tm-history-section-clear-icon" />
        </ButtonBase>
      </Box>
      <Box className="tm-history-section-list">
        {visibleRows.map(({ item, reqDevice }) => (
          <Item
            key={`${item.device_id}-${item.protocol}-${item.time}`}
            title={reqDevice.device.name}
            meta={resolveProtocolLabel(item.protocol)}
            icon={resolveDeviceOsIconBadged(reqDevice.device.os_type, 'tm-device-item-os-icon', item.protocol)}
            onClick={() => {
              void handleOpen(reqDevice, item.protocol);
            }}
          />
        ))}
        {canCollapse ? (
          <ButtonBase disableRipple className="tm-history-footer" onClick={handleToggleExpand}>
            <Box className="tm-history-item-icon" aria-hidden>
              {expanded ? <ExpandLess className="tm-history-footer-icon" /> : <ExpandMore className="tm-history-footer-icon" />}
            </Box>
            <Typography className="tm-history-footer-label">{expanded ? t('home.history.collapse') : tf('home.history.expandMore', { n: rows.length })}</Typography>
          </ButtonBase>
        ) : null}
      </Box>
    </Box>
  );
}
