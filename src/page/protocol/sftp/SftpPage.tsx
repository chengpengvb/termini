import { Box, ButtonBase } from '@mui/material';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { generateId } from '@/common/util/IdUtil.ts';
import { isNotEmpty } from '@/common/util/StringUtil.ts';
import { ReqDevice } from '@/common/model/req/ReqDevice.ts';
import { useEffect, useMemo, useRef, useState } from 'react';
import { SftpHandle } from '@/page/common/dto/sftp/SftpHandle.ts';
import { Foot } from '@/page/protocol/sftp/transfer/part/Foot.tsx';
import { SftpHome } from '@/page/protocol/sftp/transfer/SftpHome.tsx';
import { useSftpDragService } from '@/common/context/useAppHooks.tsx';
import { usePlatformMobile } from '@/common/context/usePlatformMobile';
import { usePagePanelNav } from '@/page/common/hooks/usePagePanelNav.ts';
import { SftpDragPreviewHandle } from '@/page/common/dto/sftp/SftpDrag.ts';
import { DeviceTabBarSync } from '@/page/protocol/common/DeviceTabBarSync.tsx';
import { BodyDragPreview } from '@/page/protocol/sftp/transfer/part/body/BodyDragPreview.tsx';
import { SFTP_PAGE_PANEL, type SftpPagePanel } from '@/page/common/dto/sftp/SftpHeadConst.ts';

export interface SftpProps {
  protocol: number;
  leftReqDevice: ReqDevice;
  rightReqDevice: ReqDevice;
}

export function SftpPage(props: SftpProps) {
  const { t } = useI18n();
  const dragControllerId = generateId();
  const mobileViewport = usePlatformMobile();
  const sftpDragService = useSftpDragService();
  const leftRef = useRef<SftpHandle | null>(null);
  const rightRef = useRef<SftpHandle | null>(null);
  const { leftReqDevice, rightReqDevice, protocol } = props;
  const previewRef = useRef<SftpDragPreviewHandle | null>(null);
  const [mobileSide, setMobileSide] = useState<SftpPagePanel>(SFTP_PAGE_PANEL.RIGHT);
  const { resolvePanelProps, onPanelAnimationEnd } = usePagePanelNav(mobileSide, mobileViewport);
  const dragHandlers = useMemo(() => sftpDragService.createHandle(dragControllerId, { leftRef, rightRef, previewRef }), [sftpDragService]);

  useEffect(() => {
    return () => {
      sftpDragService.disposeHandle(dragControllerId);
    };
  }, [dragControllerId, sftpDragService]);

  const pagePointerHandlers = mobileViewport
    ? null
    : {
        onPointerDown: dragHandlers.handlePointerDown,
        onPointerMove: dragHandlers.handlePointerMove,
        onPointerUp: dragHandlers.handlePointerUp,
        onPointerCancel: dragHandlers.handlePointerUp,
      };

  return (
    <Box className="tm-sftp-page" {...pagePointerHandlers}>
      <DeviceTabBarSync deviceId={rightReqDevice.device.id} protocol={protocol} />
      {mobileViewport ? (
        <Box className="tm-sftp-side-switch">
          <ButtonBase className={mobileSide === SFTP_PAGE_PANEL.LEFT ? 'tm-sftp-side-switch-btn is-active' : 'tm-sftp-side-switch-btn'} onClick={() => setMobileSide(SFTP_PAGE_PANEL.LEFT)}>
            {isNotEmpty(leftReqDevice.device.name) ? leftReqDevice.device.name : t('common.local')}
          </ButtonBase>
          <ButtonBase className={mobileSide === SFTP_PAGE_PANEL.RIGHT ? 'tm-sftp-side-switch-btn is-active' : 'tm-sftp-side-switch-btn'} onClick={() => setMobileSide(SFTP_PAGE_PANEL.RIGHT)}>
            {isNotEmpty(rightReqDevice.device.name) ? rightReqDevice.device.name : t('common.remote')}
          </ButtonBase>
        </Box>
      ) : null}
      <Box className={mobileViewport ? 'tm-sftp-panes is-mobile' : 'tm-sftp-panes'}>
        <Box {...(mobileViewport ? resolvePanelProps(SFTP_PAGE_PANEL.LEFT, 'tm-sftp-pane tm-sftp-pane-left') : { className: 'tm-sftp-pane tm-sftp-pane-left' })} onAnimationEnd={mobileViewport ? event => onPanelAnimationEnd(SFTP_PAGE_PANEL.LEFT, event) : undefined}>
          <SftpHome ref={leftRef} reqDevice={leftReqDevice} sideRef={rightRef} pathBackEnabled={!mobileViewport || mobileSide === SFTP_PAGE_PANEL.LEFT} protocol={protocol} />
        </Box>
        <Box {...(mobileViewport ? resolvePanelProps(SFTP_PAGE_PANEL.RIGHT, 'tm-sftp-pane') : { className: 'tm-sftp-pane' })} onAnimationEnd={mobileViewport ? event => onPanelAnimationEnd(SFTP_PAGE_PANEL.RIGHT, event) : undefined}>
          <SftpHome ref={rightRef} reqDevice={rightReqDevice} sideRef={leftRef} pathBackEnabled={!mobileViewport || mobileSide === SFTP_PAGE_PANEL.RIGHT} protocol={protocol} />
        </Box>
      </Box>
      <BodyDragPreview ref={previewRef} />
      <Foot leftRef={leftRef} rightRef={rightRef} />
    </Box>
  );
}
