import { RefObject } from 'react';
import { Box, IconButton } from '@mui/material';
import { EVENT_TYPE } from '../../core/types.ts';
import { useEffect, useRef, useState } from 'react';
import { inTauri } from '@/common/util/TauriUtil.ts';
import { useTabContext } from '../../hooks/useTabContext.ts';
import { ChevronLeft, ChevronRight } from '@mui/icons-material';
import { useSystemUtilService } from '@/common/context/useAppHooks.tsx';
import { PHASE_TYPE, type PaneDragState } from '../../model/PaneDrag.ts';
import type { MouseEvent as ReactMouseEvent, PointerEvent as ReactPointerEvent } from 'react';

type BarScrollProps = {
  stripRef: RefObject<HTMLDivElement | null>;
  scrollerRef: RefObject<HTMLDivElement | null>;
  onDragAreaMouseDown: (e: ReactMouseEvent<HTMLDivElement>) => void;
};

export function BarScroll(props: BarScrollProps) {
  const { manager } = useTabContext();
  const measureFrameRef = useRef(0);
  const autoScrollRafRef = useRef<number>(0);
  const [fadeRight, setFadeRight] = useState(0);
  const scrollButtonDraggingRef = useRef(false);
  const autoScrollDirRef = useRef<-1 | 0 | 1>(0);
  const systemUtilService = useSystemUtilService();
  const [canScrollLeft, setCanScrollLeft] = useState(false);
  const [canScrollRight, setCanScrollRight] = useState(false);
  const { stripRef, scrollerRef, onDragAreaMouseDown } = props;

  const measure = (): void => {
    const el = scrollerRef.current;
    const strip = stripRef.current;
    if (!el) {
      return;
    }
    setCanScrollLeft(el.scrollLeft > 1);
    setCanScrollRight(el.scrollLeft + el.clientWidth < el.scrollWidth - 1);
    if (strip) {
      setFadeRight(strip.getBoundingClientRect().right - el.getBoundingClientRect().right);
    }
  };

  const scheduleMeasure = (): void => {
    if (measureFrameRef.current > 0) {
      cancelAnimationFrame(measureFrameRef.current);
    }
    measureFrameRef.current = requestAnimationFrame(() => {
      measure();
      measureFrameRef.current = requestAnimationFrame(() => {
        measureFrameRef.current = 0;
        measure();
      });
    });
  };

  const handleScrollLeft = (): void => {
    if (scrollButtonDraggingRef.current) {
      scrollButtonDraggingRef.current = false;
      return;
    }
    scrollerRef.current?.scrollBy({ left: -220, behavior: 'smooth' });
  };

  const handleScrollRight = (): void => {
    if (scrollButtonDraggingRef.current) {
      scrollButtonDraggingRef.current = false;
      return;
    }
    scrollerRef.current?.scrollBy({ left: 220, behavior: 'smooth' });
  };

  const runAutoScrollFrame = (): void => {
    const el = scrollerRef.current;
    const dir = autoScrollDirRef.current;
    if (!el || dir === 0) {
      autoScrollRafRef.current = 0;
      return;
    }
    const SCROLL_SPEED_PX = 3;
    el.scrollLeft += dir * SCROLL_SPEED_PX;
    scheduleMeasure();
    autoScrollRafRef.current = requestAnimationFrame(runAutoScrollFrame);
  };

  const updateDragAutoScroll = (clientX: number, clientY: number): void => {
    const strip = stripRef.current;
    const scroller = scrollerRef.current;
    if (!strip || !scroller) {
      return;
    }
    const stripRect = strip.getBoundingClientRect();
    if (clientY < stripRect.top || clientY > stripRect.bottom) {
      stopDragAutoScroll();
      return;
    }
    const SCROLL_ZONE_PX = 60;
    let dir: -1 | 0 | 1 = 0;
    if (clientX <= stripRect.left + SCROLL_ZONE_PX && scroller.scrollLeft > 0) {
      dir = -1;
    } else if (clientX >= stripRect.right - SCROLL_ZONE_PX && scroller.scrollLeft + scroller.clientWidth < scroller.scrollWidth - 1) {
      dir = 1;
    }
    if (dir === autoScrollDirRef.current) {
      return;
    }
    autoScrollDirRef.current = dir;
    if (autoScrollRafRef.current !== 0) {
      cancelAnimationFrame(autoScrollRafRef.current);
      autoScrollRafRef.current = 0;
    }
    if (dir !== 0) {
      autoScrollRafRef.current = requestAnimationFrame(runAutoScrollFrame);
    }
  };

  const stopDragAutoScroll = (): void => {
    autoScrollDirRef.current = 0;
    if (autoScrollRafRef.current !== 0) {
      cancelAnimationFrame(autoScrollRafRef.current);
      autoScrollRafRef.current = 0;
    }
  };

  const armScrollButtonDrag = (pointerId: number, pointerType: string, startX: number, startY: number): void => {
    function clearListeners(): void {
      window.removeEventListener('pointermove', handlePointerMove);
      window.removeEventListener('pointerup', handlePointerUp);
      window.removeEventListener('pointercancel', handlePointerUp);
    }

    function handlePointerMove(moveEvent: PointerEvent): void {
      if (moveEvent.pointerId !== pointerId) {
        return;
      }
      const DRAG_START_DISTANCE = 5;
      const deltaX = moveEvent.clientX - startX;
      const deltaY = moveEvent.clientY - startY;
      if (deltaX * deltaX + deltaY * deltaY < DRAG_START_DISTANCE * DRAG_START_DISTANCE) {
        return;
      }
      clearListeners();
      scrollButtonDraggingRef.current = true;
      if (pointerType !== 'mouse') {
        return;
      }
      if (!inTauri()) {
        return;
      }
      void systemUtilService.startWindowDragging();
    }

    function handlePointerUp(upEvent: PointerEvent): void {
      if (upEvent.pointerId !== pointerId) {
        return;
      }
      clearListeners();
    }

    window.addEventListener('pointermove', handlePointerMove);
    window.addEventListener('pointerup', handlePointerUp);
    window.addEventListener('pointercancel', handlePointerUp);
  };

  const handleScrollButtonPointerDown = (e: ReactPointerEvent<HTMLButtonElement>): void => {
    if (e.button !== 0) {
      return;
    }
    scrollButtonDraggingRef.current = false;
    armScrollButtonDrag(e.pointerId, e.pointerType, e.clientX, e.clientY);
  };

  useEffect(() => {
    scheduleMeasure();
  }, []);

  useEffect(() => {
    const el = scrollerRef.current;
    const strip = stripRef.current;
    if (!el) {
      return;
    }
    const handleScroll = (): void => {
      scheduleMeasure();
    };
    el.addEventListener('scroll', handleScroll, { passive: true });
    const observer = new ResizeObserver(() => {
      scheduleMeasure();
    });
    observer.observe(el);
    if (strip) {
      observer.observe(strip);
    }
    return () => {
      if (measureFrameRef.current > 0) {
        cancelAnimationFrame(measureFrameRef.current);
      }
      el.removeEventListener('scroll', handleScroll);
      observer.disconnect();
    };
  }, [scrollerRef, stripRef]);

  useEffect(() => {
    return manager.addEventListener((event: number, data: any) => {
      if (EVENT_TYPE.OPEN === event || EVENT_TYPE.CLOSE === event) {
        scheduleMeasure();
        return;
      }
      if (EVENT_TYPE.DRAG !== event) {
        return;
      }
      const drag = data as PaneDragState;
      if (drag.phaseType === PHASE_TYPE.END) {
        stopDragAutoScroll();
        return;
      }
      updateDragAutoScroll(drag.clientX, drag.clientY);
    });
  }, [manager]);

  if (!canScrollLeft && !canScrollRight) {
    return null;
  }

  return (
    <Box>
      <Box className={`tm-tab-strip-fade-right${canScrollRight ? ' is-visible' : ''}`} style={{ right: fadeRight }} />
      <Box className="tm-tab-scroll-controls" onMouseDown={onDragAreaMouseDown}>
        <IconButton className="tm-btn-highlight tm-tab-scroll-btn" size="small" disabled={!canScrollLeft} onPointerDown={handleScrollButtonPointerDown} onClick={handleScrollLeft}>
          <ChevronLeft sx={{ fontSize: 17 }} />
        </IconButton>
        <IconButton className="tm-btn-highlight tm-tab-scroll-btn" size="small" disabled={!canScrollRight} onPointerDown={handleScrollButtonPointerDown} onClick={handleScrollRight}>
          <ChevronRight sx={{ fontSize: 17 }} />
        </IconButton>
      </Box>
    </Box>
  );
}
