import { translate } from '@/common/i18n/i18n.ts';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { Box, MenuItem, MenuList, Portal } from '@mui/material';
import { useEffect, useLayoutEffect, useRef, useState } from 'react';
import { usePlatformMobile } from '@/common/context/usePlatformMobile';
import type { BodyMenuContext } from '@/page/common/dto/sftp/BodyMenu.ts';
import { useOpenTransition } from '@/page/common/hooks/useOpenTransition.ts';
import { ActionSheet } from '@/page/common/components/dialog/ActionSheet.tsx';
import { clampMenuPositionForElement } from '@/page/common/util/menuPositionUtil.ts';
import { SFTP_HEAD_ACTION, SFTP_HEAD_ACTION_ITEMS } from '@/page/common/dto/sftp/SftpHeadConst.ts';

interface BodyMenuListProps {
  showMenu: boolean;
  localPane: boolean;
  showHidden: boolean;
  iosPlatform: boolean;
  chmodEnabled: boolean;
  onClose: () => void;
  renderMenuContext: BodyMenuContext | null;
  onItemClick: (code: number, context: BodyMenuContext) => void;
}

export function resolveMenuActionLabel(code: number, labelI18n: string, showHidden: boolean, mobileViewport: boolean, t: (key: string) => string): string {
  if (code === SFTP_HEAD_ACTION.TOGGLE_HIDDEN) {
    return showHidden ? t('protocol.sftp.hideHidden') : t('protocol.sftp.showHidden');
  }
  if (code === SFTP_HEAD_ACTION.COPY_TO_TARGET && mobileViewport) {
    return t('protocol.sftp.copyToPeer');
  }
  return translate(labelI18n);
}

export function BodyMenuList(props: BodyMenuListProps) {
  const { t } = useI18n();
  const mobileViewport = usePlatformMobile();
  const menuRef = useRef<HTMLDivElement | null>(null);
  const [menuPosition, setMenuPosition] = useState<{ left: number; top: number } | null>(null);
  const { showMenu, localPane, showHidden, iosPlatform, onClose, renderMenuContext, onItemClick, chmodEnabled } = props;
  const sheetOpen = Boolean(showMenu && mobileViewport);
  const { mounted: sheetMounted } = useOpenTransition(sheetOpen, 320);

  function attachMenuRef(element: HTMLDivElement | null): void {
    menuRef.current = element;
    if (!element || !renderMenuContext || mobileViewport) {
      return;
    }
    const next = clampMenuPositionForElement({ left: renderMenuContext.event.clientX, top: renderMenuContext.event.clientY, element, viewportGap: 8 });
    setMenuPosition(prev => {
      if (prev && prev.left === next.left && prev.top === next.top) {
        return prev;
      }
      return next;
    });
  }

  useLayoutEffect(() => {
    if (!renderMenuContext) {
      setMenuPosition(null);
    }
  }, [renderMenuContext]);

  useEffect(() => {
    const handlePointerDown = (event: PointerEvent): void => {
      const element = menuRef.current;
      if (!element) {
        return;
      }
      const target = event.target as Node | null;
      if (target && element.contains(target)) {
        return;
      }
      onClose();
    };
    document.addEventListener('pointerdown', handlePointerDown, true);
    return () => {
      document.removeEventListener('pointerdown', handlePointerDown, true);
    };
  }, [onClose]);

  useEffect(() => {
    if (!showMenu) {
      return;
    }
    const handleKeyDown = (event: KeyboardEvent): void => {
      if (event.key === 'Escape') {
        event.preventDefault();
        event.stopPropagation();
        onClose();
      }
    };
    document.addEventListener('keydown', handleKeyDown, true);
    return () => {
      document.removeEventListener('keydown', handleKeyDown, true);
    };
  }, [showMenu, onClose]);

  if (!renderMenuContext) {
    return null;
  }
  if (!showMenu && !sheetMounted) {
    return null;
  }

  const actionItems = SFTP_HEAD_ACTION_ITEMS.filter(item => {
    if (item.mobileOnly && !mobileViewport) {
      return false;
    }
    if (item.iosOnly === true && (!iosPlatform || !localPane)) {
      return false;
    }
    if (item.code === SFTP_HEAD_ACTION.CHMOD && !chmodEnabled) {
      return false;
    }
    return item.visible(renderMenuContext);
  }).map(item => (
    <MenuItem
      key={item.code}
      className={mobileViewport ? (item.danger ? 'tm-action-sheet-item is-danger' : 'tm-action-sheet-item') : undefined}
      onClick={() => onItemClick(item.code, renderMenuContext)}
      sx={mobileViewport ? undefined : { cursor: 'default', color: item.danger ? 'var(--tm-status-danger)' : undefined }}
    >
      {resolveMenuActionLabel(item.code, item.labelI18n, showHidden, mobileViewport, t)}
    </MenuItem>
  ));

  if (mobileViewport) {
    return (
      <ActionSheet open={showMenu} onClose={onClose}>
        <MenuList disablePadding className="tm-action-sheet-list">
          {actionItems}
        </MenuList>
      </ActionSheet>
    );
  }
  if (!showMenu) {
    return null;
  }
  return (
    <Portal>
      <Box
        ref={attachMenuRef}
        onContextMenuCapture={e => {
          e.preventDefault();
          e.stopPropagation();
        }}
        sx={{ position: 'fixed', left: menuPosition ? menuPosition.left: renderMenuContext.event.clientX, top: menuPosition ? menuPosition.top: renderMenuContext.event.clientY, zIndex: 'var(--tm-z-overlay)', minWidth: 180, py: 0.5, borderRadius: 1, bgcolor: 'var(--tm-glass-dialog-bg)', border: '1px solid var(--tm-border)', boxShadow: 'var(--tm-menu-shadow)', backdropFilter: 'var(--tm-glass-dialog-blur)', WebkitBackdropFilter: 'var(--tm-glass-dialog-blur)' }}
      >
        <MenuList dense disablePadding>
          {actionItems}
        </MenuList>
      </Box>
    </Portal>
  );
}
