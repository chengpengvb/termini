import { Box } from '@mui/material';
import { Toolbar } from './toolbar/Toolbar';
import { DeviceItem } from './item/DeviceItem';
import { isNil } from '@/common/util/StringUtil.ts';
import { useVirtualizer } from '@tanstack/react-virtual';
import { DetailPanel } from './item/detail/DetailPanel.tsx';
import type { JobDiscover } from '@/common/model/JobDiscover.ts';
import { isDiscoverRunning } from '@/common/model/JobDiscover.ts';
import { useBackDismiss } from '@/page/common/hooks/useBackDismiss.ts';
import { usePlatformMobile } from '@/common/context/usePlatformMobile';
import { useOpenTransition } from '@/page/common/hooks/useOpenTransition.ts';
import type { JobDevice, JobDevice2Indicator } from '@/common/model/JobDevice.ts';
import { JOB_MONITOR_VIEW, type JobMonitorView } from '@/page/common/model/JobModel';
import { AgentSplitDivider, SPLIT_SIDE } from '@/page/agent/common/AgentSplitDivider.tsx';
import { createDraftLink, filterJobDevice, isDraftLink } from '@/page/common/util/jobUtil.ts';
import { memo, useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react';
import { measureVirtualRow, observeAttachedElementRect } from '@/page/common/util/virtualizerRect.ts';
import { useAccountService, useJobDeviceService, useJobDiscoverService } from '@/common/context/useAppHooks.tsx';

const DeviceItemMemo = memo(DeviceItem);

export interface MonitorFocus {
  epoch: number;
  jobDeviceId: string;
  jobIndicatorId: string;
}

interface MonitorPageProps {
  listEpoch?: number;
  focus?: MonitorFocus | null;
  onDiscoverStarted?: (discover: JobDiscover) => void;
}

export function MonitorPage(props: MonitorPageProps) {
  const focusEpochRef = useRef<number>(0);
  const mobileViewport = usePlatformMobile();
  const accountService = useAccountService();
  const rootRef = useRef<HTMLDivElement>(null);
  const scrollRef = useRef<HTMLDivElement>(null);
  const drawerRef = useRef<HTMLDivElement>(null);
  const jobDeviceService = useJobDeviceService();
  const [searchText, setSearchText] = useState('');
  const jobDiscoverService = useJobDiscoverService();
  const [drawerWidth, setDrawerWidth] = useState(340);
  const [jobDevices, setJobDevices] = useState<JobDevice[]>([]);
  const [discovers, setDiscovers] = useState<JobDiscover[]>([]);
  const { listEpoch = 0, focus = null, onDiscoverStarted } = props;
  const [selectedLink, setSelectedLink] = useState<JobDevice2Indicator | null>(null);
  const [selectedJobDevice, setSelectedJobDevice] = useState<JobDevice | null>(null);
  const [view, setView] = useState<JobMonitorView>(() => jobDeviceService.getMonitorView());
  const drawerOpen = selectedLink != null;
  const { mounted: drawerMounted, active: drawerActive } = useOpenTransition(drawerOpen, 340);
  const closeDrawer = (): void => {
    setSelectedLink(null);
    setSelectedJobDevice(null);
  };
  useBackDismiss(mobileViewport && drawerOpen, closeDrawer, 'job-monitor-detail');
  const drawerCacheRef = useRef<{ link: JobDevice2Indicator; device: JobDevice } | null>(null);
  if (selectedLink && selectedJobDevice) {
    drawerCacheRef.current = { link: selectedLink, device: selectedJobDevice };
  }
  const panelLink = selectedLink ?? drawerCacheRef.current?.link ?? null;
  const panelDevice = selectedJobDevice ?? drawerCacheRef.current?.device ?? null;
  const resolvedView = mobileViewport ? JOB_MONITOR_VIEW.LIST : view;

  const runningDeviceIds = useMemo(() => {
    const ids = new Set<string>();
    for (const discover of discovers) {
      if (isDiscoverRunning(discover.status) && !isNil(discover.job_device_id)) {
        ids.add(discover.job_device_id);
      }
    }
    return ids;
  }, [discovers]);

  const keyword = searchText.trim().toLowerCase();
  const displayDevices = useMemo(() => {
    if (keyword.length === 0) {
      return jobDevices;
    }
    return jobDevices.reduce<JobDevice[]>((acc, item) => {
      const matched = filterJobDevice(item, keyword);
      if (matched != null) {
        acc.push(matched);
      }
      return acc;
    }, []);
  }, [jobDevices, keyword]);

  const rowVirtualizer = useVirtualizer({
    gap: 12,
    overscan: 4,
    estimateSize: () => 64,
    count: displayDevices.length,
    isScrollingResetDelay: 120,
    measureElement: measureVirtualRow,
    getScrollElement: () => scrollRef.current,
    observeElementRect: observeAttachedElementRect,
    getItemKey: (index: number) => displayDevices[index].id,
  });

  const handleDiscover = useCallback(
    (jobDevice: JobDevice): void => {
      void jobDiscoverService
        .handleStart(jobDevice.id)
        .then(started => {
          if (isNil(started)) {
            return;
          }
          onDiscoverStarted?.(started);
        })
        .catch(err => {
          console.error('[discover] start error', err);
        });
    },
    [jobDiscoverService, onDiscoverStarted],
  );

  const handleCloseDetail = (): void => {
    closeDrawer();
  };

  const handleDeleted = (): void => {
    handleCloseDetail();
    void jobDeviceService.selectAllDetail().then(list => {
      setJobDevices(list);
    });
  };

  const handleSaved = (saved: JobDevice2Indicator): void => {
    void jobDeviceService.selectAllDetail().then(list => {
      setJobDevices(list);
      const device = list.find(item => item.id === saved.job_device_id);
      if (device == null) {
        return;
      }
      const nextLink = device.indicators?.find(item => item.id === saved.id);
      setSelectedJobDevice(device);
      setSelectedLink(nextLink == null ? saved : nextLink);
    });
  };

  const handleViewChange = (next: JobMonitorView): void => {
    setView(next);
    jobDeviceService.setMonitorView(next);
  };

  const handleSelectIndicator = useCallback(
    (jobDevice: JobDevice, link: JobDevice2Indicator): void => {
      if (selectedLink?.id === link.id) {
        setSelectedLink(null);
        setSelectedJobDevice(null);
        return;
      }
      setSelectedLink(link);
      setSelectedJobDevice(jobDevice);
    },
    [selectedLink],
  );

  const handleAddIndicator = useCallback(
    (jobDevice: JobDevice): void => {
      if (isDraftLink(selectedLink) && selectedJobDevice?.id === jobDevice.id) {
        setSelectedLink(null);
        setSelectedJobDevice(null);
        return;
      }
      setSelectedJobDevice(jobDevice);
      setSelectedLink(createDraftLink(jobDevice));
    },
    [selectedLink, selectedJobDevice],
  );

  const handleEnabledChange = useCallback((id: string, enabled: boolean): void => {
    setJobDevices(list => list.map(item => (item.id === id ? { ...item, enabled } : item)));
  }, []);

  const handleAdded = (): void => {
    void jobDeviceService.selectAllDetail().then(setJobDevices);
  };

  useEffect(() => {
    void jobDeviceService.selectAllDetail().then(list => {
      setJobDevices(list);
    });
  }, [jobDeviceService, listEpoch]);

  useEffect(() => {
    return accountService.addCloudSyncDownloadedListener(() => {
      void jobDeviceService.selectAllDetail().then(setJobDevices);
    });
  }, [accountService, jobDeviceService]);

  useEffect(() => {
    void jobDiscoverService.selectAll().then(setDiscovers);
    return jobDiscoverService.addJobDiscoverListChangeListener(setDiscovers);
  }, [jobDiscoverService]);

  useEffect(() => {
    if (focus == null || focus.epoch === focusEpochRef.current) {
      return;
    }
    const device = jobDevices.find(item => item.id === focus.jobDeviceId);
    if (device == null) {
      return;
    }
    const link = (device.indicators ?? []).find(item => item.job_indicator_id === focus.jobIndicatorId);
    if (link == null) {
      return;
    }
    focusEpochRef.current = focus.epoch;
    setSelectedJobDevice(device);
    setSelectedLink(link);
  }, [focus, jobDevices]);

  useLayoutEffect(() => {
    if (!drawerRef.current) {
      return;
    }
    if (mobileViewport) {
      drawerRef.current.style.width = '100%';
      return;
    }
    drawerRef.current.style.width = `${drawerWidth}px`;
  }, [drawerWidth, mobileViewport]);

  return (
    <Box ref={rootRef} className="tm-job-monitor-root">
      <Box className="tm-job-page">
        <Toolbar view={resolvedView} searchText={searchText} onSearchChange={setSearchText} onViewChange={handleViewChange} onAdded={handleAdded} />
        <Box ref={scrollRef} className="tm-job-scroll">
          <Box className="tm-job-scroll-content" style={{ height: rowVirtualizer.getTotalSize() }}>
            {rowVirtualizer.getVirtualItems().map(virtualRow => {
              const jobDevice = displayDevices[virtualRow.index];
              return (
                <Box key={jobDevice.id} data-index={virtualRow.index} ref={rowVirtualizer.measureElement} className="tm-agent-list-virtual-row" style={{ transform: `translateY(${virtualRow.start}px)` }}>
                  <DeviceItemMemo
                    view={resolvedView}
                    jobDevice={jobDevice}
                    discovering={runningDeviceIds.has(jobDevice.id)}
                    selectedLinkId={selectedLink?.id ?? null}
                    addSelected={isDraftLink(selectedLink) && selectedJobDevice?.id === jobDevice.id}
                    onDiscover={handleDiscover}
                    onAddIndicator={handleAddIndicator}
                    onSelectIndicator={handleSelectIndicator}
                    onEnabledChange={handleEnabledChange}
                  />
                </Box>
              );
            })}
          </Box>
        </Box>
      </Box>
      <Box ref={drawerRef} className={drawerActive ? 'tm-job-drawer-float is-open' : 'tm-job-drawer-float'}>
        {drawerActive && !mobileViewport ? <AgentSplitDivider containerRef={rootRef} widthTargetRef={drawerRef} side={SPLIT_SIDE.RIGHT} minSize={280} minMain={440} onWidthChange={setDrawerWidth} /> : null}
        <DetailPanel open={drawerMounted} jobDevice={panelDevice} link={panelLink} onClose={handleCloseDetail} onSaved={handleSaved} onDeleted={handleDeleted} />
      </Box>
    </Box>
  );
}
