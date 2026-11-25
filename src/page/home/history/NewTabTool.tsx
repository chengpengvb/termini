import { Add } from '@mui/icons-material';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { useEffect, useRef, useState } from 'react';
import { isEmpty } from '@/common/util/StringUtil.ts';
import { Box, Popover, Typography } from '@mui/material';
import { Item } from '@/page/home/history/content/Item.tsx';
import type { ReqDevice } from '@/common/model/req/ReqDevice.ts';
import { usePlatformMobile } from '@/common/context/usePlatformMobile';
import type { HistoryLogin } from '@/common/model/dto/HistoryLogin.ts';
import { resolveDeviceOsIconBadged } from '@/page/common/util/deviceUtil.ts';
import { openDeviceProtocol, resolveProtocolLabel } from '@/page/common/util/protocolOpenUtil.ts';
import { useDeviceService, useHistoryLoginService, useTabManagerService } from '@/common/context/useAppHooks.tsx';

const RECENT_LIMIT = 5;
const SHOW_MS = 400;
const HIDE_MS = 120;

export function NewTabTool() {
  const { t } = useI18n();
  const hoverRef = useRef(false);
  const mobile = usePlatformMobile();
  const [open, setOpen] = useState(false);
  const deviceService = useDeviceService();
  const tabManagerService = useTabManagerService();
  const hideTimerRef = useRef<number | null>(null);
  const showTimerRef = useRef<number | null>(null);
  const historyLoginService = useHistoryLoginService();
  const anchorRef = useRef<HTMLDivElement | null>(null);
  const [deviceList, setDeviceList] = useState<ReqDevice[]>([]);
  const [historyList, setHistoryList] = useState<HistoryLogin[]>(() => historyLoginService.list());

  const clearHideTimer = (): void => {
    if (hideTimerRef.current === null) {
      return;
    }
    window.clearTimeout(hideTimerRef.current);
    hideTimerRef.current = null;
  };

  const clearShowTimer = (): void => {
    if (showTimerRef.current === null) {
      return;
    }
    window.clearTimeout(showTimerRef.current);
    showTimerRef.current = null;
  };

  const resolveRows = (): { item: HistoryLogin; reqDevice: ReqDevice }[] => {
    const deviceMap = new Map<string, ReqDevice>();
    for (const item of deviceList) {
      if (!item.device.id) {
        continue;
      }
      deviceMap.set(item.device.id, item);
    }
    const rows: { item: HistoryLogin; reqDevice: ReqDevice }[] = [];
    for (const item of historyList) {
      const reqDevice = deviceMap.get(item.device_id);
      if (!reqDevice || isEmpty(reqDevice.device.name)) {
        continue;
      }
      rows.push({ item, reqDevice });
      if (rows.length === RECENT_LIMIT) {
        break;
      }
    }
    return rows;
  };

  const scheduleOpen = (): void => {
    if (showTimerRef.current !== null) {
      return;
    }
    if (resolveRows().length === 0) {
      return;
    }
    showTimerRef.current = window.setTimeout(() => {
      showTimerRef.current = null;
      if (!hoverRef.current) {
        return;
      }
      if (resolveRows().length === 0) {
        return;
      }
      setOpen(true);
    }, SHOW_MS);
  };

  const handleEnter = (): void => {
    if (mobile) {
      return;
    }
    hoverRef.current = true;
    clearHideTimer();
    if (open) {
      return;
    }
    scheduleOpen();
  };

  const handleLeave = (): void => {
    if (mobile) {
      return;
    }
    hoverRef.current = false;
    clearShowTimer();
    clearHideTimer();
    hideTimerRef.current = window.setTimeout(() => {
      hideTimerRef.current = null;
      setOpen(false);
    }, HIDE_MS);
  };

  const handleClick = (): void => {
    hoverRef.current = false;
    clearShowTimer();
    setOpen(false);
  };

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
    setOpen(false);
  };

  useEffect(() => {
    const off = deviceService.addDeviceListChangeListener(list => {
      setDeviceList(list);
    });
    void deviceService.selectAllWithConfig();
    return off;
  }, [deviceService]);

  useEffect(() => {
    setHistoryList(historyLoginService.list());
    return historyLoginService.addListChangeListener(list => {
      setHistoryList(list);
    });
  }, [historyLoginService]);

  useEffect(() => {
    return () => {
      clearShowTimer();
      clearHideTimer();
    };
  }, []);

  useEffect(() => {
    if (mobile || !hoverRef.current) {
      return;
    }
    scheduleOpen();
  }, [deviceList, historyList]);

  const rows = resolveRows();
  const visible = open && rows.length > 0;

  return (
    <Box ref={anchorRef} className="tm-new-tab-tool" onMouseEnter={handleEnter} onMouseLeave={handleLeave} onClick={handleClick}>
      <Add fontSize="small" />
      <Popover
        open={visible}
        hideBackdrop
        disableScrollLock
        disableRestoreFocus
        anchorEl={anchorRef.current}
        onClose={() => setOpen(false)}
        anchorOrigin={{ vertical: 'bottom', horizontal: 'center' }}
        transformOrigin={{ vertical: 'top', horizontal: 'center' }}
        slotProps={{
          root: { sx: { pointerEvents: 'none' } },
          paper: {
            className: 'tm-new-tab-recent',
            sx: { pointerEvents: 'auto' },
            onMouseEnter: handleEnter,
            onMouseLeave: handleLeave,
          },
        }}
      >
        <Box className="tm-new-tab-recent-beak" aria-hidden>
          <svg width="14" height="7" viewBox="0 0 14 7">
            <path d="M0 7 L7 0.5 L14 7" fill="var(--tm-glass-dialog-bg)" />
            <path d="M0.5 6.8 L7 0.8 L13.5 6.8" fill="none" stroke="var(--tm-glass-dialog-border)" strokeWidth="1" strokeLinejoin="round" />
          </svg>
        </Box>
        <Typography className="tm-new-tab-recent-title">{t('tab.newPage')}</Typography>
        {rows.map(({ item, reqDevice }) => (
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
      </Popover>
    </Box>
  );
}
