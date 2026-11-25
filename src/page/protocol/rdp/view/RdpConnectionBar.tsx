import { Box, ButtonBase } from '@mui/material';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { ReactNode, useEffect, useRef, useState } from 'react';
import type { PointerEvent as ReactPointerEvent } from 'react';
import { usePlatformMobile } from '@/common/context/usePlatformMobile';
import { useSystemUtilService } from '@/common/context/useAppHooks.tsx';
import { ArrowBack, ChevronLeft, ChevronRight, Close, FilterNone, KeyboardOutlined, Mouse, PushPinOutlined, Remove, TouchApp } from '@mui/icons-material';

const SHOW_MS = 400;
const HIDE_MS = 400;
const MOBILE_AUTO_HIDE_MS = 2800;

export interface RdpConnectionBarProps {
  title: string;
  fpsOpen: boolean;
  revealTick: number;
  touchMode: boolean;
  networkBars: number;
  keyboardOpen: boolean;
  onRestore: () => void;
  onToggleFps: () => void;
  gestureHelpOpen: boolean;
  onCloseSession: () => void;
  onOpenKeyboard: () => void;
  onToggleGestureHelp: () => void;
}

export function RdpConnectionBar(props: RdpConnectionBarProps): ReactNode {
  const { t, tf } = useI18n();
  const mobileViewport = usePlatformMobile();
  const barRef = useRef<HTMLDivElement>(null);
  const rootRef = useRef<HTMLDivElement>(null);
  const titleRef = useRef<HTMLDivElement>(null);
  const systemUtilService = useSystemUtilService();
  const showTimerRef = useRef<number | null>(null);
  const hideTimerRef = useRef<number | null>(null);
  const [shiftPx, setShiftPx] = useState<number>(0);
  const [pinned, setPinned] = useState<boolean>(false);
  const [open, setOpen] = useState<boolean>(mobileViewport);
  const [macPlatform, setMacPlatform] = useState<boolean>(false);
  const dragRef = useRef<{ startX: number; startShift: number } | null>(null);
  const { title, fpsOpen, revealTick, networkBars, keyboardOpen, touchMode, gestureHelpOpen, onToggleGestureHelp, onToggleFps, onRestore, onCloseSession, onOpenKeyboard } = props;

  const clearShowTimer = (): void => {
    if (showTimerRef.current === null) {
      return;
    }
    window.clearTimeout(showTimerRef.current);
    showTimerRef.current = null;
  };

  const clearHideTimer = (): void => {
    if (hideTimerRef.current === null) {
      return;
    }
    window.clearTimeout(hideTimerRef.current);
    hideTimerRef.current = null;
  };

  const reveal = (): void => {
    clearShowTimer();
    clearHideTimer();
    setOpen(true);
  };

  const scheduleReveal = (): void => {
    clearHideTimer();
    if (pinned || open) {
      setOpen(true);
      return;
    }
    if (showTimerRef.current !== null) {
      return;
    }
    showTimerRef.current = window.setTimeout(() => {
      showTimerRef.current = null;
      setOpen(true);
    }, SHOW_MS);
  };

  const scheduleHide = (): void => {
    clearShowTimer();
    if (pinned) {
      return;
    }
    if (hideTimerRef.current !== null) {
      return;
    }
    hideTimerRef.current = window.setTimeout(() => {
      hideTimerRef.current = null;
      setOpen(false);
    }, HIDE_MS);
  };

  const revealMobile = (): void => {
    reveal();
    hideTimerRef.current = window.setTimeout(() => {
      hideTimerRef.current = null;
      setOpen(false);
    }, MOBILE_AUTO_HIDE_MS);
  };

  const handleDockToggle = (): void => {
    if (open) {
      clearShowTimer();
      clearHideTimer();
      setOpen(false);
      return;
    }
    revealMobile();
  };

  const handlePin = (): void => {
    const next = !pinned;
    setPinned(next);
    if (next) {
      reveal();
      return;
    }
    scheduleHide();
  };

  const handleMinimize = (): void => {
    void systemUtilService.minimizeWindow();
  };

  const clampShift = (next: number): number => {
    const root = rootRef.current;
    const bar = barRef.current;
    const titleEl = titleRef.current;
    const parent = root === null ? null : root.parentElement;
    if (root === null || bar === null || titleEl === null || parent === null) {
      throw new Error('rdp connect bar not mounted');
    }
    const limit = (parent.clientWidth - bar.offsetWidth) / 2 + titleEl.offsetWidth;
    if (next < -limit) {
      return -limit;
    }
    if (next > limit) {
      return limit;
    }
    return next;
  };

  const handleTitlePointerDown = (event: ReactPointerEvent<HTMLDivElement>): void => {
    if (event.button !== 0) {
      return;
    }
    event.currentTarget.setPointerCapture(event.pointerId);
    dragRef.current = { startX: event.clientX, startShift: shiftPx };
  };

  const handleTitlePointerMove = (event: ReactPointerEvent<HTMLDivElement>): void => {
    const drag = dragRef.current;
    if (!drag) {
      return;
    }
    setShiftPx(clampShift(drag.startShift + (event.clientX - drag.startX)));
  };

  const handleTitlePointerUp = (event: ReactPointerEvent<HTMLDivElement>): void => {
    if (!dragRef.current) {
      return;
    }
    dragRef.current = null;
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
  };

  useEffect(() => {
    return () => {
      clearShowTimer();
      clearHideTimer();
    };
  }, []);

  useEffect(() => {
    let cancelled = false;
    void systemUtilService.getPlatformInfo().then(info => {
      if (!cancelled) {
        setMacPlatform(info.os === 'macos');
      }
    });
    return () => {
      cancelled = true;
    };
  }, [systemUtilService]);

  useEffect(() => {
    if (revealTick === 0) {
      return;
    }
    if (mobileViewport) {
      revealMobile();
      return;
    }
    reveal();
  }, [revealTick]);

  useEffect(() => {
    if (!mobileViewport) {
      return;
    }
    revealMobile();
    return clearHideTimer;
  }, [mobileViewport]);

  const className = `tm-rdp-connect${mobileViewport ? ' is-mobile' : ''}${open ? ' is-open' : ''}${pinned ? ' is-pinned' : ''}`;

  if (mobileViewport) {
    if (keyboardOpen) {
      return null;
    }
    return (
      <Box ref={rootRef} className={className}>
        <Box ref={barRef} className="tm-rdp-connect-bar" role="toolbar" aria-label={title}>
          <ButtonBase aria-label={t('common.back')} className="tm-rdp-connect-btn" onClick={onRestore}>
            <ArrowBack fontSize="inherit" />
          </ButtonBase>
          <Box className="tm-rdp-connect-title">{title}</Box>
          <ButtonBase aria-label={t('protocol.rdp.closeSession')} className="tm-rdp-connect-btn tm-rdp-connect-btn--close" onClick={onCloseSession}>
            <Close fontSize="inherit" />
          </ButtonBase>
        </Box>
        <Box className="tm-rdp-connect-dock">
          <ButtonBase aria-label={t('protocol.rdp.showControls')} className="tm-rdp-connect-dock-btn" onClick={handleDockToggle}>
            {open ? <ChevronRight fontSize="inherit" /> : <ChevronLeft fontSize="inherit" />}
          </ButtonBase>
          <ButtonBase aria-label={t('protocol.rdp.showKeyboard')} className="tm-rdp-connect-dock-btn" onClick={onOpenKeyboard}>
            <KeyboardOutlined fontSize="inherit" />
          </ButtonBase>
          <ButtonBase aria-label={touchMode ? t('protocol.rdp.touchMode') : t('protocol.rdp.mouseMode')} className={`tm-rdp-connect-dock-btn${gestureHelpOpen ? ' is-active' : ''}`} onClick={onToggleGestureHelp}>
            {touchMode ? <TouchApp fontSize="inherit" /> : <Mouse fontSize="inherit" />}
          </ButtonBase>
        </Box>
      </Box>
    );
  }

  return (
    <Box ref={rootRef} className={className} style={{ ['--tm-rdp-connect-shift' as string]: `${shiftPx}px` }} onMouseEnter={scheduleReveal} onMouseLeave={scheduleHide}>
      <Box className="tm-rdp-connect-hotzone" onPointerDown={reveal} />
      <Box ref={barRef} className="tm-rdp-connect-bar">
        <ButtonBase aria-label={pinned ? t('protocol.rdp.unpin') : t('protocol.rdp.pin')} className={`tm-rdp-connect-btn tm-rdp-connect-btn--pin${pinned ? ' is-active' : ''}`} onClick={handlePin}>
          <PushPinOutlined fontSize="inherit" />
        </ButtonBase>
        <Box className="tm-rdp-connect-rule" />
        <Box className="tm-rdp-connect-signal" aria-label={tf('protocol.rdp.network', { bars: networkBars })}>
          <Box className={`tm-rdp-connect-signal-bar${networkBars >= 1 ? ' is-on' : ''}`} />
          <Box className={`tm-rdp-connect-signal-bar${networkBars >= 2 ? ' is-on' : ''}`} />
          <Box className={`tm-rdp-connect-signal-bar${networkBars >= 3 ? ' is-on' : ''}`} />
          <Box className={`tm-rdp-connect-signal-bar${networkBars >= 4 ? ' is-on' : ''}`} />
        </Box>
        <Box ref={titleRef} className="tm-rdp-connect-title" onPointerDown={handleTitlePointerDown} onPointerMove={handleTitlePointerMove} onPointerUp={handleTitlePointerUp} onPointerCancel={handleTitlePointerUp}>
          {title}
        </Box>
        <ButtonBase aria-label="FPS" className={`tm-rdp-connect-btn tm-rdp-connect-btn--fps${fpsOpen ? ' is-active' : ''}`} onClick={onToggleFps}>
          FPS
        </ButtonBase>
        <Box className="tm-rdp-connect-rule" />
        {macPlatform ? null : (
          <ButtonBase aria-label={t('common.minimize')} className="tm-rdp-connect-btn" onClick={handleMinimize}>
            <Remove fontSize="inherit" />
          </ButtonBase>
        )}
        <ButtonBase aria-label={t('common.restore')} className="tm-rdp-connect-btn" onClick={onRestore}>
          <FilterNone fontSize="inherit" />
        </ButtonBase>
        <ButtonBase aria-label={t('protocol.rdp.closeSession')} className="tm-rdp-connect-btn tm-rdp-connect-btn--close" onClick={onCloseSession}>
          <Close fontSize="inherit" />
        </ButtonBase>
      </Box>
    </Box>
  );
}
