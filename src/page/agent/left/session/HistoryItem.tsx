import { useI18n } from '@/common/i18n/useI18n.ts';
import type { AiChat } from '@/common/model/AiChat.ts';
import { Box, ButtonBase, Typography } from '@mui/material';
import { useNowTick } from '@/page/common/hooks/useNowTick.ts';
import { RefMsg } from '@/page/common/util/reference/RefMsg.tsx';
import { useLongPressMenu } from '@/page/common/hooks/useLongPressMenu.ts';
import { formatAgo, formatFullDateTime } from '@/page/common/util/agentTimeFormat.ts';
import { useEffect, useRef, useState, type CSSProperties, type MouseEvent, type PointerEvent as ReactPointerEvent } from 'react';

const SWIPE_ACTION_WIDTH = 72;
const AXIS_VERTICAL = 0;
const AXIS_HORIZONTAL = 1;

export interface HistoryItemProps {
  chat: AiChat;
  selected: boolean;
  swipeOpen: boolean;
  mobileSwipe: boolean;
  onSelect: (id: string) => void;
  onSwipeOpen: (id: string | null) => void;
  onDeleteClick: (e: MouseEvent<HTMLElement>, chat: AiChat) => void;
  onContextMenu: (e: MouseEvent<HTMLElement>, chat: AiChat) => void;
}

export function HistoryItem(props: HistoryItemProps) {
  const now = useNowTick();
  const offsetRef = useRef(0);
  const { t, tf } = useI18n();
  const draggedRef = useRef(false);
  const [offset, setOffset] = useState(0);
  const axisRef = useRef<number | null>(null);
  const [dragging, setDragging] = useState(false);
  const labelRef = useRef<HTMLSpanElement | null>(null);
  const startRef = useRef<{ x: number; y: number; offset: number } | null>(null);
  const [marquee, setMarquee] = useState<{ offset: number; duration: number } | null>(null);
  const { chat, selected, mobileSwipe, swipeOpen, onSelect, onSwipeOpen, onDeleteClick, onContextMenu } = props;
  const longPress = useLongPressMenu((clientX, clientY) => {
    applyOffset(0);
    onSwipeOpen(null);
    onContextMenu({preventDefault: () => undefined, stopPropagation: () => undefined, clientX, clientY} as MouseEvent<HTMLElement>, chat);
  });

  const applyOffset = (next: number): void => {
    offsetRef.current = next;
    setOffset(next);
  };

  const resolveLabelStyle = (): CSSProperties | undefined => {
    if (!marquee) {
      return undefined;
    }
    return {
      '--tm-history-marquee-x': `-${marquee.offset}px`,
      '--tm-history-marquee-duration': `${marquee.duration}s`,
    } as CSSProperties;
  };

  const resolveSwipeClass = (): string => {
    if (offset < 0 || swipeOpen) {
      return 'tm-agent-history-swipe is-open';
    }
    return 'tm-agent-history-swipe';
  };

  const handleMouseEnter = (): void => {
    const label = labelRef.current;
    const viewport = label?.parentElement;
    if (!label || !viewport) {
      return;
    }
    const overflow = label.scrollWidth - viewport.clientWidth;
    if (overflow <= 2) {
      setMarquee(null);
      return;
    }
    const duration = Math.min(8, Math.max(1.2, overflow / 40));
    setMarquee({ offset: overflow, duration });
  };

  const handleMouseLeave = (): void => {
    setMarquee(null);
  };

  const handlePointerDown = (e: ReactPointerEvent<HTMLDivElement>): void => {
    if (!mobileSwipe || (e.pointerType === 'mouse' && e.button !== 0)) {
      return;
    }
    startRef.current = { x: e.clientX, y: e.clientY, offset: offsetRef.current };
    axisRef.current = null;
    draggedRef.current = false;
    setDragging(true);
  };

  const handlePointerMove = (e: ReactPointerEvent<HTMLDivElement>): void => {
    const start = startRef.current;
    if (!start || !mobileSwipe) {
      return;
    }
    const dx = e.clientX - start.x;
    const dy = e.clientY - start.y;
    if (axisRef.current === null) {
      if (Math.abs(dx) < 6 && Math.abs(dy) < 6) {
        return;
      }
      axisRef.current = Math.abs(dx) > Math.abs(dy) ? AXIS_HORIZONTAL : AXIS_VERTICAL;
      if (axisRef.current !== AXIS_HORIZONTAL) {
        startRef.current = null;
        setDragging(false);
        return;
      }
      try {
        e.currentTarget.setPointerCapture(e.pointerId);
      } catch {}
    }
    if (axisRef.current !== AXIS_HORIZONTAL) {
      return;
    }
    draggedRef.current = true;
    applyOffset(Math.min(0, Math.max(-SWIPE_ACTION_WIDTH, start.offset + dx)));
  };

  const handlePointerUp = (): void => {
    if (!mobileSwipe) {
      return;
    }
    startRef.current = null;
    setDragging(false);
    if (axisRef.current !== AXIS_HORIZONTAL) {
      axisRef.current = null;
      return;
    }
    axisRef.current = null;
    const open = offsetRef.current <= -SWIPE_ACTION_WIDTH / 2;
    applyOffset(open ? -SWIPE_ACTION_WIDTH : 0);
    onSwipeOpen(open ? chat.id : null);
  };

  const handleSelectClick = (): void => {
    if (longPress.consumeIfFired()) {
      return;
    }
    if (draggedRef.current) {
      draggedRef.current = false;
      return;
    }
    if (mobileSwipe && offsetRef.current < 0) {
      applyOffset(0);
      onSwipeOpen(null);
      return;
    }
    onSelect(chat.id);
  };

  const handleFrontPointerDown = (e: ReactPointerEvent<HTMLDivElement>): void => {
    handlePointerDown(e);
    longPress.handlePointerDown(e);
  };

  const handleFrontPointerMove = (e: ReactPointerEvent<HTMLDivElement>): void => {
    handlePointerMove(e);
    longPress.handlePointerMove(e);
  };

  const handleFrontPointerUp = (): void => {
    handlePointerUp();
    longPress.handlePointerEnd();
  };

  const handleDeleteAction = (e: MouseEvent<HTMLElement>): void => {
    e.preventDefault();
    e.stopPropagation();
    onDeleteClick(e, chat);
    applyOffset(0);
    onSwipeOpen(null);
  };

  const resolveTitle = (): string => {
    if (chat.timestamp <= 0) {
      return chat.title;
    }
    return `${chat.title}\n${tf('agent.time.lastActive', { time: formatFullDateTime(chat.timestamp) })}`;
  };

  const resolveItem = () => (
    <ButtonBase disableRipple className={selected ? 'tm-agent-nav-item is-selected' : 'tm-agent-nav-item'} title={resolveTitle()} onClick={handleSelectClick} onContextMenu={longPress.handleContextMenu} onMouseEnter={handleMouseEnter} onMouseLeave={handleMouseLeave} onPointerDown={mobileSwipe ? undefined : longPress.handlePointerDown} onPointerMove={mobileSwipe ? undefined : longPress.handlePointerMove} onPointerUp={mobileSwipe ? undefined : longPress.handlePointerEnd} onPointerCancel={mobileSwipe ? undefined : longPress.handlePointerEnd} onPointerLeave={mobileSwipe ? undefined : longPress.handlePointerEnd}>
      <Box className="tm-agent-history-label">
        <Typography ref={labelRef} component="span" className={marquee ? 'tm-agent-nav-item-label is-marquee' : 'tm-agent-nav-item-label'} style={resolveLabelStyle()}>
          <RefMsg text={chat.title} />
        </Typography>
      </Box>
      {chat.timestamp > 0 ? (
        <Box component="time" className="tm-agent-history-time">
          {formatAgo(chat.timestamp, now)}
        </Box>
      ) : null}
    </ButtonBase>
  );

  useEffect(() => {
    if (!mobileSwipe) {
      return;
    }
    applyOffset(swipeOpen ? -SWIPE_ACTION_WIDTH : 0);
  }, [mobileSwipe, swipeOpen]);

  if (!mobileSwipe) {
    return resolveItem();
  }

  return (
    <Box className={resolveSwipeClass()} data-tm-gesture-ignore="true">
      <ButtonBase disableRipple className="tm-agent-history-swipe-delete" tabIndex={offset < 0 ? 0 : -1} onClick={handleDeleteAction}>
        {t('agent.contextMenu.delete')}
      </ButtonBase>
      <Box className={dragging ? 'tm-agent-history-swipe-front is-dragging' : 'tm-agent-history-swipe-front'} style={{ transform: `translateX(${offset}px)` }} onPointerDown={handleFrontPointerDown} onPointerMove={handleFrontPointerMove} onPointerUp={handleFrontPointerUp} onPointerCancel={handleFrontPointerUp}>
        {resolveItem()}
      </Box>
    </Box>
  );
}
