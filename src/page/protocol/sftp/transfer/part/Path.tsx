import { Box, Stack } from '@mui/material';
import Folder from '@mui/icons-material/Folder';
import { useI18n } from '@/common/i18n/useI18n.ts';
import ChevronLeft from '@mui/icons-material/ChevronLeft';
import ChevronRight from '@mui/icons-material/ChevronRight';
import { RespSftpBase } from '@/common/model/resp/RespSftp.ts';
import { useBackDismiss } from '@/page/common/hooks/useBackDismiss.ts';
import { RESP_EVENT_SFTP } from '@/common/model/resp/event/RespEventSftp.ts';
import { BodyEntriesHandle } from '@/page/common/dto/sftp/BodyEntriesHandle.ts';
import { PathEdit } from '@/page/protocol/sftp/transfer/part/path/PathEdit.tsx';
import { PathMenu } from '@/page/protocol/sftp/transfer/part/path/PathMenu.tsx';
import { useSftpService, useSystemUtilService } from '@/common/context/useAppHooks.tsx';
import { Dispatch, MouseEvent, RefObject, SetStateAction, useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react';
import { filterSftpBreadcrumbToRoot, resolveSftpBreadcrumbSegments, splitSftpPathToSegments } from '@/page/common/util/sftpUtil.ts';

export interface HeadPathProps {
  deviceId: string;
  sessionId: string;
  localPane: boolean;
  pathBackEnabled: boolean;
  setLoading: Dispatch<SetStateAction<boolean>>;
  entriesHandleRef: RefObject<BodyEntriesHandle | null>;
}

export function Path(props: HeadPathProps) {
  const { t } = useI18n();
  const sftpService = useSftpService();
  const [editing, setEditing] = useState(false);
  const homePathRef = useRef<string | null>(null);
  const systemUtilService = useSystemUtilService();
  const resetDeviceIdRef = useRef<string | null>(null);
  const [collapseLevel, setCollapseLevel] = useState(0);
  const [iosPlatform, setIosPlatform] = useState(false);
  const breadcrumbsRef = useRef<HTMLDivElement | null>(null);
  const [breadcrumbsWidth, setBreadcrumbsWidth] = useState(0);
  const [sftpPath, setSftpPath] = useState<RespSftpBase | null>(null);
  const { deviceId, sessionId, setLoading, entriesHandleRef, pathBackEnabled, localPane } = props;

  const saveLeavingPathScroll = (path: string | null | undefined): void => {
    if (!path) {
      return;
    }
    sftpService.savePathScrollTop(sessionId, path, entriesHandleRef.current?.getScrollTop() ?? 0);
  };

  const updatePath = (path: string) => {
    saveLeavingPathScroll(sftpPath?.path);
    setLoading(true);
    sftpService.handleListDir(sessionId, path);
  };

  const handleOnHomeDir = (arg: { type: number; data: any }) => {
    const respSftpPath = arg.data as RespSftpBase;
    homePathRef.current = respSftpPath.path;
    updatePath(respSftpPath.path);
  };

  const handleOnPath = (arg: { type: number; data: any }) => {
    const respSftpPath = arg.data as RespSftpBase;
    if (!sftpService.isPathHistoryNavPending(sessionId)) {
      const leavingPath = sftpService.getPathNavCurrentPath(sessionId);
      if (leavingPath && leavingPath !== respSftpPath.path) {
        saveLeavingPathScroll(leavingPath);
      }
    }
    sftpService.pushPathNav(sessionId, respSftpPath.path);
    setSftpPath(respSftpPath);
    setEditing(false);
  };

  const handleRefresh = (arg: { type: number; data: any }) => {
    const respSftpOpen = arg.data as RespSftpBase;
    let currentPath = sftpService.getCurrentPath(sessionId);
    if (respSftpOpen.path != currentPath) {
      return;
    }
    updatePath(respSftpOpen.path);
  };

  const handleEvent = (arg: { type: number; data: any }) => {
    switch (arg.type) {
      case RESP_EVENT_SFTP.ON_HOME_DIR:
        handleOnHomeDir(arg);
        break;
      case RESP_EVENT_SFTP.ON_PATH:
        handleOnPath(arg);
        break;
      case RESP_EVENT_SFTP.ON_ERROR:
        setLoading(false);
        setEditing(false);
        break;
      case RESP_EVENT_SFTP.ON_CHMODDED:
      case RESP_EVENT_SFTP.ON_RENAMED:
      case RESP_EVENT_SFTP.ON_CREATED:
      case RESP_EVENT_SFTP.ON_DELETED:
      case RESP_EVENT_SFTP.ON_TRANSFER:
        handleRefresh(arg);
        break;
      default:
        break;
    }
  };

  const handleBreadcrumbsClick = (event: MouseEvent<HTMLDivElement>): void => {
    const target = event.target as HTMLElement;
    if (target.closest('[data-tm-sftp-path-segment="true"]')) {
      return;
    }
    setEditing(true);
  };

  const handleEditSubmit = (path: string): void => {
    setEditing(false);
    updatePath(path);
  };

  const handleEditCancel = (): void => {
    setEditing(false);
  };

  const handleNavigateBack = (): void => {
    const path = sftpService.navigatePathBack(sessionId);
    if (!path) {
      return;
    }
    updatePath(path);
  };

  useBackDismiss(pathBackEnabled && sftpService.canPathNavigateBack(sessionId), handleNavigateBack, `sftp-path-${sessionId}`);

  const handleNavigateForward = (): void => {
    const path = sftpService.navigatePathForward(sessionId);
    if (!path) {
      return;
    }
    updatePath(path);
  };

  function resolveNavBackClassName(): string {
    if (sftpService.canPathNavigateBack(sessionId)) {
      return 'tm-sftp-path-nav-btn';
    }
    return 'tm-sftp-path-nav-btn tm-sftp-path-nav-btn-disabled';
  }

  function resolveNavForwardClassName(): string {
    if (sftpService.canPathNavigateForward(sessionId)) {
      return 'tm-sftp-path-nav-btn';
    }
    return 'tm-sftp-path-nav-btn tm-sftp-path-nav-btn-disabled';
  }

  useEffect(() => {
    const element = breadcrumbsRef.current;
    if (!element) {
      return;
    }
    const observer = new ResizeObserver(entries => {
      const entry = entries[0];
      if (!entry) {
        return;
      }
      setBreadcrumbsWidth(Math.floor(entry.contentRect.width));
    });
    observer.observe(element);
    return () => {
      observer.disconnect();
    };
  }, [sftpPath, editing]);

  useEffect(() => {
    const off = sftpService.addEventListener(sessionId, arg => {
      handleEvent(arg);
    });
    return () => {
      off();
    };
  }, [sessionId, sftpService]);

  useEffect(() => {
    if (resetDeviceIdRef.current === deviceId) {
      return;
    }
    resetDeviceIdRef.current = deviceId;
    homePathRef.current = null;
    setSftpPath(null);
    setEditing(false);
    sftpService.clearSessionUiState(sessionId);
  }, [deviceId, sessionId, sftpService]);

  useEffect(() => {
    let cancelled = false;
    void systemUtilService.getPlatformInfo().then(info => {
      if (!cancelled) {
        setIosPlatform(info.os === 'ios');
      }
    });
    return () => {
      cancelled = true;
    };
  }, [systemUtilService]);

  const segments = useMemo(() => {
    if (!sftpPath) {
      return [];
    }
    return splitSftpPathToSegments(sftpPath.path);
  }, [sftpPath]);

  const visibleSegments = useMemo(() => {
    const items = resolveSftpBreadcrumbSegments(segments, collapseLevel);
    if (!localPane || !iosPlatform) {
      return items;
    }
    const homePath = homePathRef.current;
    if (!homePath) {
      return items;
    }
    return filterSftpBreadcrumbToRoot(items, homePath);
  }, [collapseLevel, iosPlatform, localPane, segments]);

  useLayoutEffect(() => {
    if (!sftpPath) {
      return;
    }
    setCollapseLevel(0);
  }, [breadcrumbsWidth, sftpPath]);

  useLayoutEffect(() => {
    const element = breadcrumbsRef.current;
    if (!element || !sftpPath || collapseLevel >= 3) {
      return;
    }
    if (element.scrollWidth > element.clientWidth) {
      setCollapseLevel(value => value + 1);
    }
  }, [breadcrumbsWidth, collapseLevel, sftpPath]);

  if (!sftpPath) {
    return (
      <Box className="tm-sftp-path">
        <Box className="tm-sftp-path-loading">{t('home.common.loading')}</Box>
      </Box>
    );
  }

  return (
    <Box className="tm-sftp-path">
      <Stack direction="row" className="tm-sftp-path-nav">
        <Box className={resolveNavBackClassName()} onClick={handleNavigateBack}>
          <ChevronLeft className="tm-sftp-entry-icon-svg" />
        </Box>
        <Box className={resolveNavForwardClassName()} onClick={handleNavigateForward}>
          <ChevronRight className="tm-sftp-entry-icon-svg" />
        </Box>
      </Stack>

      {editing ? (
        <PathEdit deviceId={deviceId} initialPath={sftpPath.path} onSubmit={handleEditSubmit} onCancel={handleEditCancel} />
      ) : (
        <Box className="tm-sftp-path-breadcrumbs-wrap">
          <Stack ref={breadcrumbsRef} data-sftp-breadcrumbs="1" direction="row" className="tm-sftp-path-breadcrumbs" onClick={handleBreadcrumbsClick}>
            {visibleSegments.map((segment, index) => {
              const isLast = index === visibleSegments.length - 1;
              return (
                <Stack key={`${segment.path}-${index}`} direction="row" className="tm-sftp-path-segment">
                  {segment.isEllipsis ? (
                    <PathMenu path={segment.path}>
                      <Stack onClick={() => updatePath(segment.path)} direction="row" className="tm-sftp-path-segment-btn tm-sftp-path-segment-ellipsis" data-tm-sftp-path-segment="true">
                        <Box className="tm-sftp-path-segment-label">{segment.label}</Box>
                      </Stack>
                    </PathMenu>
                  ) : (
                    <PathMenu path={segment.path}>
                      <Stack onClick={() => updatePath(segment.path)} direction="row" className="tm-sftp-path-segment-btn" data-tm-sftp-path-segment="true">
                        <Folder className="tm-sftp-entry-icon-svg tm-sftp-entry-icon-accent tm-sftp-path-segment-icon" />
                        <Box className={isLast ? 'tm-sftp-path-segment-name tm-sftp-path-segment-name-active' : 'tm-sftp-path-segment-name'}>{segment.label}</Box>
                      </Stack>
                    </PathMenu>
                  )}
                  {!isLast ? (
                    <Box className="tm-sftp-path-separator">
                      <ChevronRight className="tm-sftp-path-separator-icon" />
                    </Box>
                  ) : null}
                </Stack>
              );
            })}
          </Stack>
          <Box className="tm-sftp-path-edit-hit" onClick={() => setEditing(true)} />
        </Box>
      )}
    </Box>
  );
}
