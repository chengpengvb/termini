import { Box } from '@mui/material';
import { PROTOCOL } from '@/common/const/Const.ts';
import { usePaneContext } from '@/common/components/tab';
import { ReqDevice } from '@/common/model/req/ReqDevice.ts';
import { RdpHandle } from '@/page/common/dto/rdp/RdpHandle.ts';
import { RdpView } from '@/page/protocol/rdp/view/RdpView.tsx';
import { RdpOpenEvent } from '@/common/model/req/event/WsRdp.ts';
import { EVENT_TYPE } from '@/common/components/tab/core/types.ts';
import { StepPage } from '@/page/protocol/common/step/StepPage.tsx';
import { RdpAuthPage } from '@/page/protocol/rdp/auth/RdpAuthPage.tsx';
import { usePlatformMobile } from '@/common/context/usePlatformMobile';
import { useDocumentClass } from '@/page/common/hooks/useSoftKeyboardOpen.ts';
import { DeviceTabBarSync } from '@/page/protocol/common/DeviceTabBarSync.tsx';
import { ReactNode, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { createRdpFullscreenAction } from '@/page/protocol/rdp/RdpFullscreenAction.tsx';
import { useRdpService, useSystemUtilService, useTabManagerService } from '@/common/context/useAppHooks.tsx';

export interface RdpPageProps {
  reqDevice: ReqDevice;
}

type ChromeRestore = {
  sidePanelVisible: boolean;
  windowFullscreen: boolean;
};

export function RdpPage(props: RdpPageProps): ReactNode {
  const { reqDevice } = props;
  const rdpService = useRdpService();
  const { paneId } = usePaneContext();
  const rdpRef = useRef<RdpHandle>(null);
  const openedRef = useRef<boolean>(false);
  const mobileViewport = usePlatformMobile();
  const tabManagerService = useTabManagerService();
  const systemUtilService = useSystemUtilService();
  const [opened, setOpened] = useState<boolean>(false);
  const rdpOpenRef = useRef<(() => void) | null>(null);
  const nativeMobile = useDocumentClass('tm-native-mobile');
  const chromeRestoreRef = useRef<ChromeRestore | null>(null);
  const [chromeHidden, setChromeHidden] = useState<boolean>(false);

  const enterFullscreen = useCallback(async (): Promise<void> => {
    if (document.body.classList.contains('tm-rdp-chrome-hidden')) {
      return;
    }
    const tab = tabManagerService.getTabs().find(item => item.panes.some(pane => pane.id === paneId));
    if (!tab || !tab.id) {
      throw new Error(`RDP fullscreen: tab owning pane ${paneId} not found`);
    }
    if (tabManagerService.getActiveTabId() !== tab.id) {
      tabManagerService.setActiveTabId(tab.id);
    }
    const sidePanelVisible = tabManagerService.isSidePanelVisible();
    const platform = await systemUtilService.getPlatformInfo();
    if (platform.mobile || mobileViewport) {
      chromeRestoreRef.current = { sidePanelVisible, windowFullscreen: false };
      document.body.classList.add('tm-rdp-chrome-hidden');
      tabManagerService.setSidePanelVisible(false);
      setChromeHidden(true);
      return;
    }
    const windowFullscreen = await systemUtilService.getWindowFullscreen();
    chromeRestoreRef.current = { sidePanelVisible, windowFullscreen };
    document.body.classList.add('tm-rdp-chrome-hidden');
    tabManagerService.setSidePanelVisible(false);
    setChromeHidden(true);
    if (!windowFullscreen) {
      await systemUtilService.setWindowFullscreen(true);
    }
  }, [mobileViewport, paneId, systemUtilService, tabManagerService]);

  const exitFullscreen = useCallback(async (): Promise<void> => {
    const restore = chromeRestoreRef.current;
    if (!restore) {
      return;
    }
    chromeRestoreRef.current = null;
    document.body.classList.remove('tm-rdp-chrome-hidden');
    setChromeHidden(false);
    tabManagerService.setSidePanelVisible(restore.sidePanelVisible);
    const platform = await systemUtilService.getPlatformInfo();
    if (platform.mobile) {
      return;
    }
    await systemUtilService.setWindowFullscreen(restore.windowFullscreen);
  }, [systemUtilService, tabManagerService]);

  const fullscreenAction = useMemo(
    () =>
      createRdpFullscreenAction(() => {
        void enterFullscreen();
      }),
    [enterFullscreen],
  );

  const init = async () => {
    if (!rdpRef.current) {
      return;
    }
    rdpRef.current.init(() => {
      doConnect(false).then();
    });
  };

  const doConnect = async (force: boolean) => {
    if (openedRef.current && !force) {
      return;
    }
    const rdp = rdpRef.current;
    if (!rdp) {
      return;
    }
    openedRef.current = true;
    destroyRdp();
    const size = rdp.getWindowSize();
    const payload: RdpOpenEvent = { data: { deviceId: reqDevice.device.id, size: size } };
    rdpOpenRef.current = await rdpService.handleConnect(paneId, payload, rdp, setOpened);
  };

  const destroyAll = (): void => {
    destroyRdp();
    rdpRef.current?.dispose();
  };

  const destroyRdp = (): void => {
    rdpOpenRef.current?.();
    rdpOpenRef.current = null;
  };

  useEffect(() => {
    const unsubscribe = tabManagerService.addPaneOpenListener(pane => {
      if (paneId !== pane.id) {
        return;
      }
      init().then();
    });
    return () => {
      unsubscribe();
    };
  }, [paneId, tabManagerService]);

  useEffect(() => {
    const unsubscribe = rdpService.addConnectRetryListener(paneId, () => {
      doConnect(true).then();
    });
    return () => {
      unsubscribe();
    };
  }, [paneId, rdpService]);

  useEffect(() => {
    return tabManagerService.addEventListener((event: number) => {
      if (event !== EVENT_TYPE.ACTIVE) {
        return;
      }
      const tab = tabManagerService.getTabs().find(item => item.panes.some(pane => pane.id === paneId));
      if (!tab || tabManagerService.getActiveTabId() !== tab.id) {
        void exitFullscreen();
        return;
      }
      if (nativeMobile) {
        void enterFullscreen();
      }
    });
  }, [enterFullscreen, exitFullscreen, nativeMobile, paneId, tabManagerService]);

  useEffect(() => {
    if (!nativeMobile) {
      return;
    }
    void enterFullscreen();
  }, [enterFullscreen, nativeMobile]);

  useEffect(() => {
    return () => {
      const restore = chromeRestoreRef.current;
      if (!restore) {
        return;
      }
      chromeRestoreRef.current = null;
      document.body.classList.remove('tm-rdp-chrome-hidden');
      tabManagerService.setSidePanelVisible(restore.sidePanelVisible);
      void systemUtilService.getPlatformInfo().then(platform => {
        if (platform.mobile) {
          return;
        }
        void systemUtilService.setWindowFullscreen(restore.windowFullscreen);
      });
    };
  }, [systemUtilService, tabManagerService]);

  return (
    <Box sx={{ position: 'relative', width: '100%', height: '100%' }}>
      <DeviceTabBarSync deviceId={reqDevice.device.id} protocol={PROTOCOL.RDP} {...fullscreenAction} />
      <RdpView
        deviceId={reqDevice.device.id}
        ref={rdpRef}
        title={reqDevice.device.name}
        connected={opened}
        chromeHidden={chromeHidden}
        onExitFullscreen={() => {
          void exitFullscreen();
        }}
        closePane={destroyAll}
      ></RdpView>
      <StepPage protocol={PROTOCOL.RDP} connectService={rdpService} reqDevice={reqDevice} sessionId={paneId} authPage={<RdpAuthPage connectService={rdpService} reqDevice={reqDevice} sessionId={paneId} />} />
    </Box>
  );
}
