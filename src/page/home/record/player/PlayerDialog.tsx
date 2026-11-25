import '@xterm/xterm/css/xterm.css';
import type { PointerEvent } from 'react';
import { Terminal as XTerm } from '@xterm/xterm';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { useEffect, useRef, useState } from 'react';
import { Pause, PlayArrow, Replay } from '@mui/icons-material';
import { formatSeconds } from '@/page/common/util/recordFormat.ts';
import { Box, ButtonBase, Dialog, Typography } from '@mui/material';
import { Header } from '@/page/common/components/dialog/Header.tsx';
import type { SessionRecord } from '@/common/model/SessionRecord.ts';
import { resolveErrorText } from '@/common/model/resp/ResultModel.ts';
import { ActivityWave } from '@/page/home/record/player/ActivityWave.tsx';
import { useCastPlayer } from '@/page/home/record/player/useCastPlayer.ts';
import { useSessionRecordService } from '@/common/context/useAppHooks.tsx';
import { queryXtermScreen, resolveTerminalFontFamily, resolveTerminalTheme, stampXtermScreen } from '@/page/common/util/terminal.ts';

const MIN_FONT_SIZE = 9;
const MAX_FONT_SIZE = 28;

export function PlayerDialog(props: { open: boolean; record: SessionRecord | null; onClose: () => void }) {
  const { t, tf } = useI18n();
  const player = useCastPlayer();
  const { open, record, onClose } = props;
  const stageRef = useRef<HTMLDivElement | null>(null);
  const sessionRecordService = useSessionRecordService();
  const [error, setError] = useState<string | null>(null);
  const progressRef = useRef<HTMLDivElement | null>(null);
  const [container, setContainer] = useState<HTMLDivElement | null>(null);
  const dragRef = useRef<{ active: boolean; wasPlaying: boolean }>({ active: false, wasPlaying: false });
  const progressRatio = player.total > 0 ? Math.min(1, player.elapsed / player.total) : 0;

  function resolvePointerSeconds(clientX: number): number {
    const bar = progressRef.current;
    if (bar === null || player.total <= 0) {
      return 0;
    }
    const rect = bar.getBoundingClientRect();
    const ratio = Math.min(1, Math.max(0, (clientX - rect.left) / rect.width));
    return ratio * player.total;
  }

  function handleProgressPointerDown(event: PointerEvent<HTMLDivElement>): void {
    if (player.total <= 0) {
      return;
    }
    dragRef.current = { active: true, wasPlaying: player.playing };
    event.currentTarget.setPointerCapture(event.pointerId);
    player.previewSeek(resolvePointerSeconds(event.clientX));
  }

  function handleProgressPointerMove(event: PointerEvent<HTMLDivElement>): void {
    if (!dragRef.current.active) {
      return;
    }
    player.previewSeek(resolvePointerSeconds(event.clientX));
  }

  function handleProgressPointerUp(event: PointerEvent<HTMLDivElement>): void {
    if (!dragRef.current.active) {
      return;
    }
    const wasPlaying = dragRef.current.wasPlaying;
    dragRef.current = { active: false, wasPlaying: false };
    player.seekTo(resolvePointerSeconds(event.clientX), wasPlaying);
  }

  useEffect(() => {
    if (!open || !record || !container) {
      return;
    }
    const stage = stageRef.current;
    if (stage === null) {
      return;
    }
    let disposed = false;
    const terminal = new XTerm({
      cols: record.width,
      rows: record.height,
      disableStdin: true,
      cursorBlink: false,
      fontSize: 14,
      lineHeight: 1,
      letterSpacing: 0,
      fontFamily: resolveTerminalFontFamily(),
      scrollback: 10000,
      theme: resolveTerminalTheme(),
    });
    terminal.open(container);
    stampXtermScreen(container);
    player.attachTerminal(terminal);

    const stageStyle = window.getComputedStyle(stage);
    const stagePadX = parseFloat(stageStyle.paddingLeft) + parseFloat(stageStyle.paddingRight);
    const stagePadY = parseFloat(stageStyle.paddingTop) + parseFloat(stageStyle.paddingBottom);
    const stageInnerW = stage.clientWidth - stagePadX;
    const stageInnerH = stage.clientHeight - stagePadY;
    let fitRafId = 0;
    const measureScale = (): number | null => {
      const screen = queryXtermScreen(container);
      if (screen === null || screen.offsetWidth <= 0 || screen.offsetHeight <= 0) {
        return null;
      }
      return Math.min(stageInnerW / screen.offsetWidth, stageInnerH / screen.offsetHeight);
    };
    const shrinkUntilFit = (): void => {
      const scale = measureScale();
      const current = terminal.options.fontSize ?? 14;
      if (scale === null) {
        return;
      }
      if (scale >= 1 || current <= MIN_FONT_SIZE) {
        return;
      }
      terminal.options.fontSize = current - 1;
      fitRafId = window.requestAnimationFrame(shrinkUntilFit);
    };
    const applyFit = (): void => {
      const scale = measureScale();
      if (scale === null) {
        return;
      }
      const current = terminal.options.fontSize ?? 14;
      const target = Math.min(MAX_FONT_SIZE, Math.max(MIN_FONT_SIZE, Math.floor(current * scale)));
      if (target !== current) {
        terminal.options.fontSize = target;
      }
      fitRafId = window.requestAnimationFrame(shrinkUntilFit);
    };
    fitRafId = window.requestAnimationFrame(applyFit);

    sessionRecordService
      .content(record.id)
      .then(content => {
        if (disposed) {
          return;
        }
        setError(null);
        player.load(content);
      })
      .catch(err => {
        if (disposed) {
          return;
        }
        setError(resolveErrorText(err));
      });

    return () => {
      disposed = true;
      window.cancelAnimationFrame(fitRafId);
      player.reset();
      player.attachTerminal(null);
      terminal.dispose();
      setError(null);
    };
  }, [open, record, container, sessionRecordService]);

  if (!record) {
    return null;
  }

  return (
    <Dialog open={open} onClose={onClose} maxWidth={false} slotProps={{ paper: { className: 'tm-record-player-paper' } }}>
      <Box className="tm-record-player-body" data-tm-overlay="true">
        <Header title={`${record.device_name} · ${record.protocol.toUpperCase()}`} onClose={onClose} />
        <Box ref={stageRef} className="tm-record-player-stage">
          <Box ref={setContainer} className="tm-record-player-xterm" data-tm-terminal="true" />
          {error ? <Typography className="tm-record-player-error">{tf('home.common.loadFailed', { error })}</Typography> : null}
        </Box>
        <Box className="tm-record-player-controls">
          <Box
            ref={progressRef}
            role="slider"
            aria-valuemin={0}
            aria-valuemax={Math.round(player.total)}
            aria-valuenow={Math.round(player.elapsed)}
            aria-label={t('home.record.seek')}
            className="tm-record-player-progress"
            onPointerDown={handleProgressPointerDown}
            onPointerMove={handleProgressPointerMove}
            onPointerUp={handleProgressPointerUp}
            onPointerCancel={handleProgressPointerUp}
          >
            <ActivityWave activity={player.activity} ratio={progressRatio} />
            <Box className="tm-record-player-progress-head" sx={{ left: `${progressRatio * 100}%` }} />
          </Box>
          <Box className="tm-record-player-toolbar">
            <ButtonBase disableRipple className="tm-record-player-btn" onClick={player.togglePlay} aria-label={player.playing ? t('home.record.pause') : t('home.record.play')}>
              {player.playing ? <Pause /> : <PlayArrow />}
            </ButtonBase>
            <ButtonBase disableRipple className="tm-record-player-btn" onClick={player.restart} aria-label={t('home.record.replay')}>
              <Replay />
            </ButtonBase>
            <Typography className="tm-record-player-time">
              {formatSeconds(player.elapsed)} / {formatSeconds(player.total)}
            </Typography>
            <Box className="tm-record-player-toolbar-spacer" />
            <ButtonBase disableRipple className="tm-record-player-speed" onClick={player.cycleSpeed}>
              {player.speed}x
            </ButtonBase>
          </Box>
        </Box>
      </Box>
    </Dialog>
  );
}
