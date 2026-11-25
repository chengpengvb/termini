import { useRef, useState } from 'react';
import { useI18n } from '@/common/i18n/useI18n.ts';
import type { Device } from '@/common/model/Device.ts';
import { isNotEmpty } from '@/common/util/StringUtil.ts';
import { Box, ButtonBase, Typography } from '@mui/material';
import type { PointerEvent as ReactPointerEvent } from 'react';
import type { RespTree } from '@/common/model/resp/RespTree.ts';
import type { ReqDevice } from '@/common/model/req/ReqDevice.ts';
import { CHAT_TARGET_TYPE } from '@/common/const/ChatTargetConst.ts';
import { resolveErrorText } from '@/common/model/resp/ResultModel.ts';
import { armReferenceDrag } from '@/page/common/util/reference/drag.ts';
import { ChevronRight, Cable, Delete, Edit } from '@mui/icons-material';
import { AlertDialog } from '@/common/components/dialog/AlertDialog.tsx';
import type { ChatReference } from '@/common/model/dto/ChatReference.ts';
import { useLongPressMenu } from '@/page/common/hooks/useLongPressMenu.ts';
import { ConfirmDialog } from '@/common/components/dialog/ConfirmDialog.tsx';
import { SelectProtocol } from '@/page/protocol/common/select/SelectProtocol.tsx';
import { CommonMenu, CommonMenuPosition } from '@/common/components/menu/CommonMenu.tsx';
import { useDeviceService, useTabManagerService } from '@/common/context/useAppHooks.tsx';
import { resolveDeviceOsIconBadged, resolveDeviceTabIcon } from '@/page/common/util/deviceUtil.ts';

const MENU_ITEM = { CONNECT: 0, EDIT: 1, DELETE: 2 } as const;

export function DeviceTreeNode(props: { node: RespTree; expandedIds: Record<string, boolean>; onToggleGroup: (groupId: string) => void; setReqDevice: (reqDevice: ReqDevice) => void; setDrawerVisible: (visible: boolean) => void }) {
  const { t, tf } = useI18n();
  const { node, expandedIds, onToggleGroup, setReqDevice, setDrawerVisible } = props;
  const deviceService = useDeviceService();
  const tabManagerService = useTabManagerService();
  const dragOffRef = useRef<(() => void) | null>(null);
  const [showAlert, setShowAlert] = useState<boolean>(false);
  const pressStartRef = useRef<{ x: number; y: number } | null>(null);
  const [confirmDeleteOpen, setConfirmDeleteOpen] = useState<boolean>(false);
  const [menuPosition, setMenuPosition] = useState<CommonMenuPosition | null>(null);
  const [alertMessage, setAlertMessage] = useState<string>(t('home.common.dataLoadFailed'));
  const longPress = useLongPressMenu((clientX, clientY) => {
    if (node.type === CHAT_TARGET_TYPE.GROUP) {
      return;
    }
    if (dragOffRef.current) {
      dragOffRef.current();
      dragOffRef.current = null;
    }
    setMenuPosition({ top: clientY, left: clientX });
  });

  const isGroup = (): boolean => {
    return node.type === CHAT_TARGET_TYPE.GROUP;
  };

  const isGroupExpanded = (): boolean => {
    return expandedIds[node.id];
  };

  const resolveDeviceIcon = () => {
    if (!node.data) {
      return null;
    }
    const device: Device = JSON.parse(node.data);
    return resolveDeviceOsIconBadged(device.os_type, 'tm-agent-nav-item-icon');
  };

  const buildReference = (): ChatReference => {
    if (!node.data) {
      return { kind: node.type, id: node.id, title: node.title, data: null };
    }
    return { kind: node.type, id: node.id, title: node.title, data: JSON.parse(node.data) };
  };

  const handleClick = (): void => {
    if (longPress.consumeIfFired()) {
      return;
    }
    if (!isGroup()) {
      return;
    }
    onToggleGroup(node.id);
  };

  const handlePointerDown = (e: ReactPointerEvent<HTMLElement>): void => {
    if (e.button !== 0) {
      return;
    }
    e.preventDefault();
    pressStartRef.current = { x: e.clientX, y: e.clientY };
    if (!isGroup()) {
      longPress.handlePointerDown(e);
    }
    dragOffRef.current = armReferenceDrag(buildReference(), e.pointerId, e.clientX, e.clientY);
  };

  const handlePointerMove = (e: ReactPointerEvent<HTMLElement>): void => {
    const start = pressStartRef.current;
    if (start) {
      const dx = e.clientX - start.x;
      const dy = e.clientY - start.y;
      if (dx * dx + dy * dy >= 25) {
        longPress.handlePointerEnd();
        pressStartRef.current = null;
      }
    }
    if (isGroup()) {
      return;
    }
    longPress.handlePointerMove(e);
  };

  const handlePointerEnd = (): void => {
    pressStartRef.current = null;
    if (isGroup()) {
      return;
    }
    longPress.handlePointerEnd();
  };

  const handleCloseMenu = (): void => {
    setMenuPosition(null);
  };

  const handleConnect = async (): Promise<void> => {
    try {
      const loaded = await deviceService.selectWithConfig(node.id);
      if (!loaded) {
        setAlertMessage(t('home.common.dataLoadFailed'));
        setShowAlert(true);
        return;
      }
      tabManagerService.openTab({
        panes: [
          {
            tabBar: {
              canClose: true,
              title: loaded.device.name,
              icon: resolveDeviceTabIcon(loaded.device.os_type),
            },
            content: <SelectProtocol reqDevice={loaded} />,
          },
        ],
      });
    } catch (err) {
      setAlertMessage(resolveErrorText(err));
      setShowAlert(true);
    }
  };

  const handleEdit = async (): Promise<void> => {
    try {
      const loaded = await deviceService.selectWithConfig(node.id);
      if (!loaded) {
        setAlertMessage(t('home.common.dataLoadFailed'));
        setShowAlert(true);
        return;
      }
      setReqDevice(loaded);
      setDrawerVisible(true);
    } catch (err) {
      setAlertMessage(resolveErrorText(err));
      setShowAlert(true);
    }
  };

  const handleCancelDelete = (): void => {
    setConfirmDeleteOpen(false);
  };

  const handleConfirmDelete = async (): Promise<void> => {
    setConfirmDeleteOpen(false);
    await deviceService.delete(node.id);
  };

  const handleDelete = (): void => {
    if (!isNotEmpty(node.id)) {
      return;
    }
    setConfirmDeleteOpen(true);
  };

  const resolveChildNodes = (): RespTree[] | null => {
    if (!isGroup()) {
      return null;
    }
    if (!node.children) {
      return null;
    }
    if (node.children.length === 0) {
      return null;
    }
    return node.children;
  };

  const childNodes = resolveChildNodes();

  return (
    <Box className="tm-agent-nav-tree">
      <ButtonBase
        disableRipple
        className={isGroup() ? 'tm-agent-nav-item is-group' : 'tm-agent-nav-item is-device'}
        onClick={handleClick}
        onContextMenu={isGroup() ? undefined : longPress.handleContextMenu}
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={handlePointerEnd}
        onPointerCancel={handlePointerEnd}
        onPointerLeave={handlePointerEnd}
      >
        {isGroup() ? null : (
          <Box className="tm-agent-nav-item-icon-slot" aria-hidden>
            {resolveDeviceIcon()}
          </Box>
        )}
        <Typography component="span" className="tm-agent-nav-item-label">
          {node.title}
        </Typography>
        {isGroup() ? <ChevronRight className={isGroupExpanded() ? 'tm-agent-nav-item-chevron is-expanded' : 'tm-agent-nav-item-chevron'} /> : null}
      </ButtonBase>
      {childNodes ? (
        <Box className={isGroupExpanded() ? 'tm-agent-nav-tree-children is-open' : 'tm-agent-nav-tree-children'}>
          <Box className="tm-agent-nav-tree-children-inner">
            {childNodes.map(child => (
              <DeviceTreeNode key={child.id} node={child} expandedIds={expandedIds} onToggleGroup={onToggleGroup} setReqDevice={setReqDevice} setDrawerVisible={setDrawerVisible} />
            ))}
          </Box>
        </Box>
      ) : null}
      {isGroup() ? null : (
        <>
          <AlertDialog duration={1500} open={showAlert} handleClose={() => setShowAlert(false)} message={alertMessage} type="error" />
          <CommonMenu
            position={menuPosition}
            onClose={handleCloseMenu}
            items={[
              {
                key: MENU_ITEM.CONNECT,
                text: t('home.common.connect'),
                icon: <Cable />,
                onClick: () => {
                  void handleConnect();
                },
              },
              {
                key: MENU_ITEM.EDIT,
                text: t('common.edit'),
                icon: <Edit />,
                onClick: () => {
                  void handleEdit();
                },
              },
              { key: MENU_ITEM.DELETE, text: t('common.delete'), icon: <Delete />, danger: true, onClick: handleDelete },
            ]}
          />
          <ConfirmDialog
            open={confirmDeleteOpen}
            title={t('home.devices.deleteTitle')}
            message={tf('home.devices.deleteMessage', { name: node.title })}
            confirmLabel={t('common.delete')}
            danger
            onCancel={handleCancelDelete}
            onConfirm={() => {
              void handleConfirmDelete();
            }}
          />
        </>
      )}
    </Box>
  );
}
