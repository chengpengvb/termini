import { Box } from '@mui/material';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { generateId } from '@/common/util/IdUtil.ts';
import { inTauri } from '@/common/util/TauriUtil.ts';
import { PhysicalPosition } from '@tauri-apps/api/dpi';
import { usePaneContext } from '@/common/components/tab';
import { ReqDevice } from '@/common/model/req/ReqDevice.ts';
import { closeTerminal } from '@/page/common/util/terminal.ts';
import { RespSftpError } from '@/common/model/resp/RespSftp.ts';
import { AuthPage } from '@/page/protocol/ssh/auth/AuthPage.tsx';
import { DEVICE_CONNECTION_TYPE } from '@/common/model/Device.ts';
import { SftpHandle } from '@/page/common/dto/sftp/SftpHandle.ts';
import { SftpOpenEvent } from '@/common/model/req/event/WsSftp.ts';
import { DragFromSelection } from '@/page/common/dto/sftp/SftpDrag';
import { StepPage } from '@/page/protocol/common/step/StepPage.tsx';
import { SftpAlertState } from '@/page/common/dto/sftp/SftpAlert.ts';
import { AlertDialog } from '@/common/components/dialog/AlertDialog.tsx';
import { DragDropEvent, getCurrentWebview } from '@tauri-apps/api/webview';
import { RESP_EVENT_SFTP } from '@/common/model/resp/event/RespEventSftp.ts';
import { SftpTransfer } from '@/page/protocol/sftp/transfer/SftpTransfer.tsx';
import { HeadSelect } from '@/page/protocol/sftp/transfer/part/head/HeadSelect.tsx';
import { Ref, RefObject, useEffect, useImperativeHandle, useRef, useState } from 'react';
import { BodyEntriesHandle, BodyEntriesVirtualHandle } from '@/page/common/dto/sftp/BodyEntriesHandle.ts';
import { useDeviceService, useSftpService, useTabManagerService } from '@/common/context/useAppHooks.tsx';
import { groupPathsByParent, buildOsDropTransferPayloads, isPointInsideElement, physicalPositionToCssPoint, resolveWebviewDragDevicePixelRatio } from '@/page/common/util/sftpUtil.ts';

export interface TransferHomeProps {
  protocol: number;
  reqDevice: ReqDevice;
  ref?: Ref<SftpHandle>;
  pathBackEnabled: boolean;
  sideRef?: RefObject<SftpHandle | null>;
}

export function SftpHome(props: TransferHomeProps) {
  const { t } = useI18n();
  const { paneId } = usePaneContext();
  const sftpService = useSftpService();
  const openedRef = useRef<boolean>(false);
  const deviceService = useDeviceService();
  const panelRef = useRef<SftpHandle | null>(null);
  const tabManagerService = useTabManagerService();
  const [sessionId] = useState<string>(generateId());
  const [opened, setOpened] = useState<boolean>(false);
  const rootRef = useRef<HTMLDivElement | null>(null);
  const sftpOpenRef = useRef<(() => void) | null>(null);
  const activeReqDeviceRef = useRef<ReqDevice>(props.reqDevice);
  const [showHidden, setShowHidden] = useState<boolean>(false);
  const bodyEntriesHandleRef = useRef<BodyEntriesHandle | null>(null);
  const sftpEntriesRef = useRef<BodyEntriesVirtualHandle | null>(null);
  const { reqDevice, ref, sideRef, pathBackEnabled, protocol } = props;
  const [dragCoverVisible, setDragCoverVisible] = useState<boolean>(false);
  const [headSelectVisible, setHeadSelectVisible] = useState<boolean>(false);
  const [activeReqDevice, setActiveReqDevice] = useState<ReqDevice>(reqDevice);
  const isLocalType = activeReqDevice.device.type === DEVICE_CONNECTION_TYPE.LOCAL;
  const [alertState, setAlertState] = useState<SftpAlertState>({ open: false, message: null });

  const doConnect = async (force: boolean): Promise<void> => {
    if (openedRef.current && !force) {
      return;
    }
    openedRef.current = true;
    destroySftp();
    const device = activeReqDeviceRef.current.device;
    const payload: SftpOpenEvent = { data: { deviceId: device.id, deviceType: device.type, protocol } };
    sftpOpenRef.current = await sftpService.handleConnect(sessionId, payload, null, setOpened);
  };

  const destroySftp = (): void => {
    sftpOpenRef.current?.();
    sftpOpenRef.current = null;
    sftpService.clearSessionUiState(sessionId);
  };

  const getSessionId = (): string => {
    return sessionId;
  };

  const showDragCover = (): void => {
    setDragCoverVisible(true);
  };

  const isLocal = (): boolean => {
    return isLocalType;
  };

  const closePane = (): void => {
    destroySftp();
  };

  const hideDragCover = (): void => {
    setDragCoverVisible(false);
  };

  const clearSelection = (): void => {
    sftpEntriesRef.current?.clearSelection();
  };

  const getRootElement = (): HTMLElement | null => {
    return rootRef.current;
  };

  const getSelectedNames = (): string[] => {
    return sftpEntriesRef.current?.getSelectedNames() || [];
  };

  const getCurrentPath = (): string | undefined => {
    return sftpService.getCurrentPath(sessionId);
  };

  const getClientTransferId = (): string | undefined => {
    return sftpService.getClientTransferId(sessionId);
  };

  const updateDragSelect = (clientX: number, clientY: number): void => {
    sftpEntriesRef.current?.updateDragSelect(clientX, clientY);
  };

  const endDragSelect = (clientX: number, clientY: number): void => {
    sftpEntriesRef.current?.endDragSelect(clientX, clientY);
  };

  const cancelDragSelect = (): void => {
    sftpEntriesRef.current?.cancelDragSelect();
  };

  const beginDragSelect = (clientX: number, clientY: number, append: boolean, onFromSelection?: (ctx: DragFromSelection) => void): void => {
    sftpEntriesRef.current?.beginDragSelect(clientX, clientY, append, onFromSelection);
  };

  const setFilterQuery = (query: string | null): void => {
    bodyEntriesHandleRef.current?.setFilterQuery(query);
  };

  const openActionsMenu = (clientX: number, clientY: number): void => {
    bodyEntriesHandleRef.current?.openActionsMenu(clientX, clientY);
  };

  const openSearch = (): void => {
    bodyEntriesHandleRef.current?.openSearch();
  };

  const openHeadSelect = (): void => {
    setHeadSelectVisible(true);
  };

  const closeHeadSelect = (): void => {
    setHeadSelectVisible(false);
  };

  const handleAlertClose = (): void => {
    setAlertState({ open: false, message: null });
  };

  const handleOnError = (data: any): void => {
    const error = data as RespSftpError;
    if (!error.message) {
      setAlertState({ open: true, message: t('result.error') });
      return;
    }
    const sandbox = error.message.includes('path is outside the local documents directory');
    setAlertState({ open: true, message: sandbox ? t('protocol.sftp.localDocumentsOnly') : error.message });
  };

  const handleHeadSelectConnect = async (deviceId: string): Promise<void> => {
    if (deviceId === activeReqDeviceRef.current.device.id) {
      setHeadSelectVisible(false);
      return;
    }
    const loaded = await deviceService.selectWithConfig(deviceId);
    if (!loaded) {
      return;
    }
    setHeadSelectVisible(false);
    activeReqDeviceRef.current = loaded;
    setActiveReqDevice(loaded);
    setOpened(false);
    await doConnect(true);
  };

  const dispatchOsDropTransfers = (targetPath: string, sourceClientTransferId: string, targetClientTransferId: string, pathsByParent: Map<string, string[]>): void => {
    const payloads = buildOsDropTransferPayloads(sessionId, targetPath, sourceClientTransferId, targetClientTransferId, pathsByParent);
    for (const payload of payloads) {
      sftpService.handleTransfer(sessionId, payload);
    }
  };

  const handleOsFileDropAsync = async (paths: string[]): Promise<void> => {
    if (!opened || paths.length === 0) {
      return;
    }
    const currentPath = sftpService.getCurrentPath(sessionId);
    if (!currentPath) {
      return;
    }
    const sourceClientTransferId = await sftpService.getLocalClientTransferId();
    if (!sourceClientTransferId) {
      return;
    }
    const targetClientTransferId = getClientTransferId();
    if (!targetClientTransferId) {
      return;
    }
    const pathsByParent = groupPathsByParent(paths);
    dispatchOsDropTransfers(currentPath, sourceClientTransferId, targetClientTransferId, pathsByParent);
  };

  const handleOsFileDrop = (paths: string[]): void => {
    handleOsFileDropAsync(paths).then();
  };

  const applyDragOverlayHover = (position: PhysicalPosition, dpr: number): void => {
    const point = physicalPositionToCssPoint(position, dpr);
    setDragCoverVisible(isPointInsideElement(rootRef.current, point.x, point.y));
  };

  const applyDragOverlayLeave = (): void => {
    setDragCoverVisible(false);
  };

  const applyDragOverlayDrop = (position: PhysicalPosition, paths: string[], dpr: number): void => {
    setDragCoverVisible(false);
    const point = physicalPositionToCssPoint(position, dpr);
    if (!isPointInsideElement(rootRef.current, point.x, point.y)) {
      return;
    }
    handleOsFileDrop(paths);
  };

  const handleWebviewDragDropEvent = (payload: DragDropEvent): void => {
    const dpr = resolveWebviewDragDevicePixelRatio();
    if (payload.type === 'enter' || payload.type === 'over') {
      applyDragOverlayHover(payload.position, dpr);
      return;
    }
    if (payload.type === 'leave') {
      applyDragOverlayLeave();
      return;
    }
    if (payload.type === 'drop') {
      applyDragOverlayDrop(payload.position, payload.paths, dpr);
    }
  };

  useImperativeHandle(ref, () => {
    const handle: SftpHandle = {
      isLocal,
      showDragCover,
      hideDragCover,
      getSessionId,
      endDragSelect,
      getCurrentPath,
      clearSelection,
      getRootElement,
      setFilterQuery,
      beginDragSelect,
      openSearch,
      openActionsMenu,
      updateDragSelect,
      cancelDragSelect,
      getSelectedNames,
      getClientTransferId,
    };
    panelRef.current = handle;
    return handle;
  });

  useEffect(() => {
    if (!opened) {
      return;
    }
    if (!inTauri()) {
      return;
    }
    let unlisten: (() => void) | undefined;
    getCurrentWebview()
      .onDragDropEvent(event => {
        handleWebviewDragDropEvent(event.payload);
      })
      .then(fn => {
        unlisten = fn;
      });
    return () => {
      unlisten?.();
    };
  }, [opened, sessionId, sftpService, isLocalType, sideRef]);

  useEffect(() => {
    const unsubscribe = tabManagerService.addPaneOpenListener(pane => {
      if (paneId !== pane.id) {
        return;
      }
      doConnect(false).then();
    });
    return () => {
      unsubscribe();
    };
  }, [paneId, tabManagerService]);

  useEffect(() => {
    const unsubscribe = sftpService.addConnectRetryListener(sessionId, () => {
      doConnect(true).then();
    });
    return () => {
      unsubscribe();
    };
  }, [sessionId, sftpService]);

  useEffect(() => {
    const off = sftpService.addEventListener(sessionId, arg => {
      if (arg.type !== RESP_EVENT_SFTP.ON_ERROR) {
        return;
      }
      handleOnError(arg.data);
    });
    return () => {
      off();
    };
  }, [sessionId, sftpService]);

  useEffect(() => {
    const unsubscribe = tabManagerService.addCloseGuardListener((dragMode: number, data: any) => {
      closeTerminal(paneId, dragMode, data, closePane);
      return true;
    });
    return () => {
      unsubscribe();
    };
  }, [paneId, sessionId, sftpService, tabManagerService]);

  return (
    <Box ref={rootRef} className="tm-sftp-home" sx={{ position: 'relative', width: '100%', height: '100%', overflow: 'hidden' }}>
      <SftpTransfer
        reqDevice={activeReqDevice}
        sessionId={sessionId}
        sideRef={sideRef}
        panelRef={panelRef}
        virtualRef={sftpEntriesRef}
        entriesHandleRef={bodyEntriesHandleRef}
        dragCoverVisible={dragCoverVisible}
        showHidden={showHidden}
        setShowHidden={setShowHidden}
        dialogContainerRef={rootRef}
        onIdentityClick={openHeadSelect}
        pathBackEnabled={pathBackEnabled}
        protocol={protocol}
      />
      {isLocalType ? null : <StepPage key={activeReqDevice.device.id} protocol={protocol} connectService={sftpService} reqDevice={activeReqDevice} sessionId={sessionId} authPage={<AuthPage connectService={sftpService} reqDevice={activeReqDevice} sessionId={sessionId} />} />}
      {headSelectVisible ? (
        <HeadSelect
          excludeDeviceId={activeReqDevice.device.id}
          onClose={closeHeadSelect}
          onConnect={deviceId => {
            handleHeadSelectConnect(deviceId).then();
          }}
        />
      ) : null}
      <AlertDialog duration={2500} open={alertState.open} handleClose={handleAlertClose} message={alertState.message} type="error" containerRef={rootRef} />
    </Box>
  );
}
