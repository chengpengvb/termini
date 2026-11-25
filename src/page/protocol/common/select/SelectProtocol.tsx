import { Box } from '@mui/material';
import { useEffect, useRef, useState } from 'react';
import { usePaneContext } from '@/common/components/tab';
import type { ReqDevice } from '@/common/model/req/ReqDevice.ts';
import { DeviceHead } from '@/page/protocol/common/select/DeviceHead.tsx';
import { DeviceTabBarSync } from '@/page/protocol/common/DeviceTabBarSync.tsx';
import { ProtocolContext } from '@/page/protocol/common/select/ProtocolContext.tsx';
import { openDeviceProtocol, resolveSelectableProtocols } from '@/page/common/util/protocolOpenUtil.ts';
import { useDeviceService, useHistoryLoginService, useTabManagerService } from '@/common/context/useAppHooks.tsx';

export interface SelectPageProps {
  reqDevice: ReqDevice;
}

export function SelectProtocol(props: SelectPageProps) {
  const { reqDevice } = props;
  const { paneId } = usePaneContext();
  const autoConnectedRef = useRef(false);
  const deviceService = useDeviceService();
  const tabManagerService = useTabManagerService();
  const historyLoginService = useHistoryLoginService();
  const [loaded, setLoaded] = useState<ReqDevice | null>(null);

  const openProtocol = async (protocol: number, device: ReqDevice): Promise<void> => {
    await openDeviceProtocol({
      protocol,
      reqDevice: device,
      tabManagerService,
      resolveLocalDevice: () => deviceService.localDevice(),
    });
  };

  const doConnect = async (protocols: number[], device: ReqDevice): Promise<void> => {
    const time = Date.now();
    for (let i = 0; i < protocols.length; i += 1) {
      historyLoginService.add(device.device.id, protocols[i], time);
      await openProtocol(protocols[i], device);
    }
    tabManagerService.closePane(paneId);
  };

  useEffect(() => {
    let cancelled = false;
    deviceService.selectWithConfig(reqDevice.device.id).then(data => {
      if (cancelled) {
        return;
      }
      if (!data) {
        console.warn('[protocol] selectWithConfig returned no data');
        return;
      }
      setLoaded(data);
    });
    return () => {
      cancelled = true;
    };
  }, [reqDevice.device.id, deviceService]);

  useEffect(() => {
    if (!loaded || autoConnectedRef.current) {
      return;
    }
    const protocols = resolveSelectableProtocols(loaded);
    if (protocols.length !== 1) {
      return;
    }
    autoConnectedRef.current = true;
    void doConnect(protocols, loaded);
  }, [loaded]);

  if (!loaded) {
    return null;
  }

  return (
    <Box sx={{ position: 'relative', width: '100%', height: '100%' }}>
      <DeviceTabBarSync deviceId={loaded.device.id} />
      <Box sx={{ position: 'absolute', inset: 0 }}>
        <Box className="tm-protocol-frame-root">
          <Box className="tm-protocol-frame tm-protocol-frame--select">
            <DeviceHead reqDevice={loaded} />
            <Box className="tm-protocol-frame-body">
              <ProtocolContext doConnect={protocols => doConnect(protocols, loaded)} reqDevice={loaded} />
            </Box>
          </Box>
        </Box>
      </Box>
    </Box>
  );
}
