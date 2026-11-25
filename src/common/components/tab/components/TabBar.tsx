import { Box } from '@mui/material';
import type { ReactNode } from 'react';
import { BarItems } from './tabBar/BarItems.tsx';
import { BarTools } from './tabBar/BarTools.tsx';
import { BarScroll } from './tabBar/BarScroll.tsx';
import { inTauri } from '@/common/util/TauriUtil.ts';
import { BarControls } from './tabBar/BarControls.tsx';
import { useTabContext } from '../hooks/useTabContext';
import { HAPTIC_KIND } from '@/common/model/dto/Haptic.ts';
import type { MouseEvent as ReactMouseEvent } from 'react';
import { usePlatformMobile } from '@/common/context/usePlatformMobile';
import { useSystemUtilService } from '@/common/context/useAppHooks.tsx';
import { useCallback, useEffect, useLayoutEffect, useRef, useState } from 'react';
import { applyFlipInverse, playFlipForward, snapshotElementRects } from '../util/flipAnimation';
import { BAR_TOOL_SLOT, EVENT_TYPE, type TabConfig, type TabLayoutEventData } from '../core/types';

export function TabBar(): ReactNode {
  const { manager } = useTabContext();
  const mobileViewport = usePlatformMobile();
  const systemUtilService = useSystemUtilService();
  const stripRef = useRef<HTMLDivElement | null>(null);
  const scrollerRef = useRef<HTMLDivElement | null>(null);
  const tabItemElsRef = useRef<Map<string, HTMLElement>>(new Map());
  const flipSnapshotRef = useRef<Map<string, DOMRect> | null>(null);
  const [tabs, setTabs] = useState<TabConfig[]>(() => manager.getTabs());

  const tabsRef = useRef<TabConfig[]>(tabs);
  tabsRef.current = tabs;
  const activeTabIdRef = useRef<string | undefined>(manager.getActiveTabId());

  const setItemEl = useCallback((tabId: string, el: HTMLElement | null): void => {
    if (el) {
      tabItemElsRef.current.set(tabId, el);
      return;
    }
    tabItemElsRef.current.delete(tabId);
  }, []);

  const handleDragAreaMouseDown = useCallback(
    (e: ReactMouseEvent<HTMLDivElement>): void => {
      if (e.button !== 0 || e.target !== e.currentTarget) {
        return;
      }
      e.preventDefault();
      if (!inTauri()) {
        return;
      }
      if (e.detail >= 2) {
        void systemUtilService.toggleMaximizeWindow();
        return;
      }
      void systemUtilService.startWindowDragging();
    },
    [systemUtilService],
  );

  const triggerTabHaptic = useCallback((): void => {
    if (!mobileViewport) {
      return;
    }
    void systemUtilService.haptic({ kind: HAPTIC_KIND.SELECTION });
  }, [mobileViewport, systemUtilService]);

  const handleManagerEvent = useCallback(
    (event: number, data: any): void => {
      if (EVENT_TYPE.ACTIVE === event) {
        const nextActiveTabId = data as string | undefined;
        const previousActiveTabId = activeTabIdRef.current;
        activeTabIdRef.current = nextActiveTabId;
        if (nextActiveTabId && nextActiveTabId !== previousActiveTabId) {
          triggerTabHaptic();
        }
        return;
      }
      if (EVENT_TYPE.OPEN !== event && EVENT_TYPE.CLOSE !== event && EVENT_TYPE.LAYOUT !== event) {
        return;
      }
      if (EVENT_TYPE.OPEN === event || EVENT_TYPE.CLOSE === event) {
        flipSnapshotRef.current = snapshotElementRects(tabItemElsRef.current);
        const isTabAdded = EVENT_TYPE.OPEN === event && (data as TabConfig[]).length > tabsRef.current.length;
        const isTabClosed = EVENT_TYPE.CLOSE === event;
        if (isTabAdded || isTabClosed) {
          triggerTabHaptic();
        }
      }
      if (EVENT_TYPE.LAYOUT === event) {
        setTabs((data as TabLayoutEventData).tabs);
        return;
      }
      setTabs(data as TabConfig[]);
    },
    [triggerTabHaptic],
  );

  useLayoutEffect(() => {
    const snapshot = flipSnapshotRef.current;
    if (!snapshot) {
      return;
    }
    flipSnapshotRef.current = null;
    const targets = applyFlipInverse(tabItemElsRef.current, snapshot);
    if (targets.length === 0) {
      return;
    }
    window.requestAnimationFrame(() => {
      playFlipForward(targets);
    });
  });

  useEffect(() => {
    return manager.addEventListener(handleManagerEvent);
  }, [manager, handleManagerEvent]);

  return (
    <Box className="tm-tab-bar">
      <Box className="tm-tab-strip" ref={stripRef} onMouseDown={handleDragAreaMouseDown}>
        <BarItems tabs={tabs} stripRef={stripRef} tabsRef={tabsRef} scrollerRef={scrollerRef} tabItemElsRef={tabItemElsRef} setItemEl={setItemEl} onDragAreaMouseDown={handleDragAreaMouseDown} />
        <BarScroll stripRef={stripRef} scrollerRef={scrollerRef} onDragAreaMouseDown={handleDragAreaMouseDown} />
      </Box>
      <BarTools slot={BAR_TOOL_SLOT.RIGHT} />
      <BarControls />
    </Box>
  );
}
