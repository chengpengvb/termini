import { Box } from '@mui/material';
import { usePaneContext } from '@/common/components/tab';
import {focusRdpCanvas} from '@/page/common/util/rdpUtil.ts';
import { RdpHandle } from '@/page/common/dto/rdp/RdpHandle.ts';
import { useRdpFrame } from '@/page/common/hooks/useRdpFrame.ts';
import { useRdpInput } from '@/page/common/hooks/useRdpInput.ts';
import { useRdpTouch } from '@/page/common/hooks/useRdpTouch.ts';
import type { RdpFocus } from '@/common/model/req/event/WsRdp.ts';
import { useRdpCursor } from '@/page/common/hooks/useRdpCursor.ts';
import { useRdpLayout } from '@/page/common/hooks/useRdpLayout.ts';
import { RdpPointer } from '@/page/protocol/rdp/view/RdpPointer.tsx';
import { RdpJoystick } from '@/page/protocol/rdp/view/RdpJoystick.tsx';
import { RdpKeyboard } from '@/page/protocol/rdp/view/RdpKeyboard.tsx';
import { usePlatformMobile } from '@/common/context/usePlatformMobile';
import { useTabManagerService } from '@/common/context/useAppHooks.tsx';
import { RdpGestureHelp } from '@/page/protocol/rdp/view/RdpGestureHelp.tsx';
import {useRdpSoftKeyboard} from '@/page/common/hooks/useRdpSoftKeyboard.ts';
import { closeTerminal, requestSoftInput } from '@/page/common/util/terminal.ts';
import { RdpConnectionBar } from '@/page/protocol/rdp/view/RdpConnectionBar.tsx';
import { useDocumentClass, useSoftKeyboardOpen } from '@/page/common/hooks/useSoftKeyboardOpen.ts';
import {readRdpMobilePreferences, saveRdpMobilePreferences, type RdpMobileQuality} from '@/page/common/util/rdpMobilePreferences.ts';
import { Ref, useEffect, useImperativeHandle, useRef, useState, type FormEvent, type KeyboardEvent as ReactKeyboardEvent } from 'react';

export interface RdpViewProps {
  deviceId?: string;
  title: string;
  ref?: Ref<RdpHandle>;
  connected: boolean;
  chromeHidden: boolean;
  onExitFullscreen: () => void;
  closePane?: (dragMode: number, data: any) => void;
}

export function RdpView(props: RdpViewProps) {
  const { paneId } = usePaneContext();
  const frameCountRef = useRef<number>(0);
  const [fps, setFps] = useState<number>(0);
  const mobileViewport = usePlatformMobile();
  const composingRef = useRef<boolean>(false);
  const hostRef = useRef<HTMLDivElement>(null);
  const pointerRef = useRef<HTMLDivElement>(null);
  const softKeyboardOpen = useSoftKeyboardOpen();
  const tabManagerService = useTabManagerService();
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const imeRef = useRef<HTMLTextAreaElement>(null);
  const hardKeyboard = useDocumentClass('tm-hard-keyboard');
  const hardMouse = useDocumentClass('tm-hard-mouse');
  const [fpsOpen, setFpsOpen] = useState<boolean>(false);
  const [revealTick, setRevealTick] = useState<number>(0);
  const [networkBars, setNetworkBars] = useState<number>(0);
  const [touchMode, setTouchMode] = useState(() => readRdpMobilePreferences(props.deviceId).touchMode);
  const [quality, setQuality] = useState<RdpMobileQuality>(() => readRdpMobilePreferences(props.deviceId).quality);
  const desktopPointer = !mobileViewport || hardMouse;
  const [gestureHelpOpen, setGestureHelpOpen] = useState<boolean>(false);
  const focusCallbackRef = useRef<((focus: RdpFocus | null) => void) | null>(null);
  const { ref, title, connected, chromeHidden, onExitFullscreen, closePane } = props;
  const touchSurface = mobileViewport && !desktopPointer;
  const keyboard = useRdpSoftKeyboard(canvasRef, imeRef, mobileViewport, softKeyboardOpen, requestSoftInput);

  const cursor = useRdpCursor(canvasRef);
  const input = useRdpInput(
    paneId,
    touchSurface,
    canvasRef,
  );
  const contextRef = useRef<CanvasRenderingContext2D | null>(null);
  const frame = useRdpFrame(canvasRef, contextRef);
  const layout = useRdpLayout(hostRef, canvasRef, contextRef, paneId, mobileViewport, quality);
  const touch = useRdpTouch(canvasRef, input.sendEvents, pointerRef, touchSurface, touchMode, focus => focusCallbackRef.current?.(focus));
  useEffect(() => {
    if (mobileViewport) {
      saveRdpMobilePreferences(props.deviceId, {touchMode, quality});
    }
  }, [mobileViewport, props.deviceId, touchMode, quality]);

  const drawFrame = (payload: ArrayBuffer): Promise<number> => {
    frameCountRef.current += 1;
    return frame.drawFrame(payload);
  };

  const onFocusChange = (cb: (focus: RdpFocus | null) => void): (() => void) => {
    focusCallbackRef.current = cb;
    return () => {
      if (focusCallbackRef.current === cb) {
        focusCallbackRef.current = null;
      }
    };
  };

  const handleToggleFps = (): void => {
    setFpsOpen(!fpsOpen);
  };

  const handleCloseSession = (): void => {
    onExitFullscreen();
    tabManagerService.closePane(paneId);
  };

  const flushIme = (): void => {
    const el = imeRef.current;
    if (el === null) {
      throw new Error('rdp ime missing');
    }
    const text = el.value;
    el.value = '';
    if (text.length === 0) {
      return;
    }
    input.sendText(text);
  };

  const handleOpenKeyboard = (): void => {
    keyboard.open();
    setRevealTick(tick => tick + 1);
  };

  const handleHideKeyboard = (): void => {
    keyboard.hide();
  };

  const handleImeKeyDown = (event: ReactKeyboardEvent<HTMLTextAreaElement>): void => {
    event.stopPropagation();
    if (event.nativeEvent.isComposing || event.key === 'Process') {
      return;
    }
    if (event.ctrlKey || event.metaKey || event.altKey || event.key === 'Backspace' || event.key === 'Enter' || event.key === 'Tab' || event.key === 'Escape' || event.key.startsWith('Arrow') || event.key.startsWith('F')) {
      event.preventDefault();
      input.sendKey(event.nativeEvent);
    }
  };

  const handleImeCompositionStart = (): void => {
    composingRef.current = true;
  };

  const handleImeCompositionEnd = (): void => {
    composingRef.current = false;
    flushIme();
  };

  const handleImeInput = (): void => {
    if (composingRef.current) {
      return;
    }
    flushIme();
  };

  const handleImeBeforeInput = (event: FormEvent<HTMLTextAreaElement>): void => {
    const inputEvent = event.nativeEvent as InputEvent;
    if (inputEvent.inputType !== 'deleteContentBackward' && inputEvent.inputType !== 'deleteContentForward') {
      return;
    }
    event.preventDefault();
    input.sendCode(inputEvent.inputType === 'deleteContentBackward' ? 'Backspace' : 'Delete');
  };

  const dispose = (): void => {
    layout.dispose();
    cursor.dispose();
    input.dispose();
    frame.dispose();
    contextRef.current = null;
  };

  useImperativeHandle(ref, () => {
    const handle: RdpHandle = {
      dispose,
      drawFrame,
      init: layout.init,
      onData: input.onData,
      onFocusChange,
      setNetwork: setNetworkBars,
      setCursor: cursor.setCursor,
      resizeSurface: frame.resizeSurface,
      getWindowSize: layout.getWindowSize,
      onWindowResize: layout.onWindowResize,
      onVisibilityChange: layout.onVisibilityChange,
    };
    const objectRef = ref as {current: RdpHandle | null} | null | undefined;
    if (objectRef !== null && objectRef !== undefined && objectRef.current !== null && objectRef.current !== undefined) {
      Object.assign(objectRef.current, handle);
      return objectRef.current;
    }
    return handle;
  });

  useEffect(() => {
    if (!fpsOpen) {
      return;
    }
    frameCountRef.current = 0;
    const id = window.setInterval(() => {
      setFps(frameCountRef.current);
      frameCountRef.current = 0;
    }, 1000);
    return () => {
      window.clearInterval(id);
    };
  }, [fpsOpen]);

  useEffect(() => {
    if (!connected || !hardKeyboard) {
      return;
    }
    const canvas = canvasRef.current;
    if (canvas) {
      focusRdpCanvas(canvas);
    }
  }, [connected, hardKeyboard]);

  useEffect(() => {
    const unsubscribe = tabManagerService.addCloseGuardListener((dragMode: number, data: any) => {
      closeTerminal(paneId, dragMode, data, closePane);
      return true;
    });
    return () => {
      unsubscribe();
    };
  }, [paneId, tabManagerService]);

  return (
    <Box ref={hostRef} className="tm-rdp-shell" data-tm-rdp="true" onContextMenu={event => event.preventDefault()}>
      <canvas ref={canvasRef} tabIndex={0} inputMode="none" className="tm-rdp-canvas" draggable={false} onContextMenu={event => event.preventDefault()} />
      {fpsOpen ? <Box className="tm-rdp-fps">{fps} fps</Box> : null}
      {touchSurface ? <RdpPointer ref={pointerRef} kind={cursor.kind} image={cursor.image} visible={touch.cursorReady && connected} /> : null}
      {touchSurface && connected && !touchMode ? <RdpJoystick onMove={touch.moveCursor} /> : null}
      {mobileViewport && chromeHidden && keyboard.requested ? <RdpKeyboard send={input.sendEvents} onHide={handleHideKeyboard} onFit={touch.fit} /> : null}
      {mobileViewport && chromeHidden && gestureHelpOpen ? <RdpGestureHelp touchMode={touchMode} onTouchModeChange={setTouchMode} quality={quality} onQualityChange={setQuality} onFit={touch.fit} onActualSize={touch.actualSize} onScroll={touch.scroll} onClose={() => setGestureHelpOpen(false)} /> : null}
      {chromeHidden ? (
        <RdpConnectionBar
          title={title}
          fpsOpen={fpsOpen}
          networkBars={networkBars}
          revealTick={revealTick}
          keyboardOpen={keyboard.requested}
          touchMode={touchMode}
          gestureHelpOpen={gestureHelpOpen}
          onToggleGestureHelp={() => setGestureHelpOpen(open => !open)}
          onToggleFps={handleToggleFps}
          onRestore={onExitFullscreen}
          onCloseSession={handleCloseSession}
          onOpenKeyboard={handleOpenKeyboard}
        />
      ) : null}
      <textarea
        ref={imeRef}
        className="tm-rdp-ime"
        readOnly={!keyboard.requested}
        inputMode={keyboard.requested ? 'text' : 'none'}
        tabIndex={-1}
        data-tm-ime="true"
        autoCapitalize="off"
        autoCorrect="off"
        spellCheck={false}
        enterKeyHint="enter"
        aria-hidden="true"
        onCompositionStart={handleImeCompositionStart}
        onCompositionEnd={handleImeCompositionEnd}
        onBeforeInput={handleImeBeforeInput}
        onInput={handleImeInput}
        onKeyDown={handleImeKeyDown}
      />
    </Box>
  );
}
