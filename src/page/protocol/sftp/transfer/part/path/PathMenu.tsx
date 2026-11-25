import { useI18n } from '@/common/i18n/useI18n.ts';
import { Box, MenuItem, MenuList, Portal } from '@mui/material';
import { CommonMenu } from '@/common/components/menu/CommonMenu.tsx';
import { usePlatformMobile } from '@/common/context/usePlatformMobile';
import { useSystemUtilService } from '@/common/context/useAppHooks.tsx';
import { useLongPressMenu } from '@/page/common/hooks/useLongPressMenu.ts';
import { clampMenuPositionForElement } from '@/page/common/util/menuPositionUtil.ts';
import { MouseEvent, ReactNode, useCallback, useEffect, useRef, useState } from 'react';

interface PathMenuProps {
  path: string;
  children: ReactNode;
}

interface MenuAnchor {
  left: number;
  top: number;
}

export function PathMenu(props: PathMenuProps) {
  const { t } = useI18n();
  const { path, children } = props;
  const mobileViewport = usePlatformMobile();
  const systemUtilService = useSystemUtilService();
  const menuRef = useRef<HTMLDivElement | null>(null);
  const [anchor, setAnchor] = useState<MenuAnchor | null>(null);
  const [menuPosition, setMenuPosition] = useState<{ left: number; top: number } | null>(null);

  const closeMenu = (): void => {
    setAnchor(null);
    setMenuPosition(null);
  };

  const openAt = (clientX: number, clientY: number): void => {
    setAnchor({ left: clientX, top: clientY });
  };

  const openMenu = (event: MouseEvent<HTMLElement>): void => {
    event.preventDefault();
    event.stopPropagation();
    openAt(event.clientX, event.clientY);
  };

  const longPress = useLongPressMenu((clientX, clientY) => {
    openAt(clientX, clientY);
  });

  const attachMenuRef = useCallback(
    (element: HTMLDivElement | null): void => {
      menuRef.current = element;
      if (!element || !anchor) {
        return;
      }
      setMenuPosition(clampMenuPositionForElement({ left: anchor.left, top: anchor.top, element, viewportGap: 8 }));
    },
    [anchor],
  );

  const handleCopyPath = async (): Promise<void> => {
    if (path) {
      await systemUtilService.writeClipboardText(path);
    }
    closeMenu();
  };

  const handleTriggerClickCapture = (event: MouseEvent<HTMLElement>): void => {
    if (longPress.consumeIfFired()) {
      event.preventDefault();
      event.stopPropagation();
    }
  };

  useEffect(() => {
    if (!anchor || mobileViewport) {
      return;
    }
    const handlePointerDown = (event: PointerEvent): void => {
      const target = event.target as Node | null;
      const element = menuRef.current;
      if (target && element && element.contains(target)) {
        return;
      }
      closeMenu();
    };
    const handleKeyDown = (event: KeyboardEvent): void => {
      if (event.key === 'Escape') {
        closeMenu();
      }
    };
    document.addEventListener('pointerdown', handlePointerDown, true);
    document.addEventListener('keydown', handleKeyDown, true);
    return () => {
      document.removeEventListener('pointerdown', handlePointerDown, true);
      document.removeEventListener('keydown', handleKeyDown, true);
    };
  }, [anchor]);

  return (
    <>
      <Box
        component="span"
        className="tm-sftp-path-menu-trigger"
        onContextMenu={openMenu}
        onClickCapture={handleTriggerClickCapture}
        onPointerDown={longPress.handlePointerDown}
        onPointerMove={longPress.handlePointerMove}
        onPointerUp={longPress.handlePointerEnd}
        onPointerCancel={longPress.handlePointerEnd}
      >
        {children}
      </Box>
      {mobileViewport ? (
        <CommonMenu
          position={anchor ? { top: anchor.top, left: anchor.left } : null}
          items={[
            {
              key: 1,
              text: t('protocol.sftp.copyPath'),
              onClick: () => {
                void handleCopyPath();
              },
            },
          ]}
          onClose={closeMenu}
        />
      ) : null}
      {!mobileViewport && anchor ? (
        <Portal>
          <Box
            ref={attachMenuRef}
            className={menuPosition ? 'tm-sftp-path-menu' : 'tm-sftp-path-menu tm-sftp-path-menu-hidden'}
            style={{ left: menuPosition ? menuPosition.left : anchor.left, top: menuPosition ? menuPosition.top : anchor.top }}
            onClick={event => {
              event.stopPropagation();
            }}
            onContextMenuCapture={event => {
              event.preventDefault();
              event.stopPropagation();
            }}
          >
            <MenuList dense disablePadding>
              <MenuItem
                onClick={event => {
                  event.stopPropagation();
                  void handleCopyPath();
                }}
                className="tm-sftp-path-menu-item"
              >
                {t('protocol.sftp.copyPath')}
              </MenuItem>
            </MenuList>
          </Box>
        </Portal>
      ) : null}
    </>
  );
}
