import { useEffect, useState } from 'react';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { CommonMenu } from '@/common/components/menu/CommonMenu.tsx';
import { usePlatformMobile } from '@/common/context/usePlatformMobile';
import { Box, ClickAwayListener, MenuItem, MenuList, Portal } from '@mui/material';
import { clampMenuPositionForElement } from '@/page/common/util/menuPositionUtil.ts';

const MENU_MIN_WIDTH = 180;

interface Position {
  left: number;
  top: number;
}

export interface ProjectContextMenuProps {
  origin: Position;
  onClose: () => void;
  onRename: () => void;
  onDelete: () => void;
}

export function ProjectContextMenu(props: ProjectContextMenuProps) {
  const { t } = useI18n();
  const mobileViewport = usePlatformMobile();
  const { origin, onRename, onDelete, onClose } = props;
  const [position, setPosition] = useState<Position | null>(null);

  const attachMenu = (element: HTMLDivElement | null): void => {
    if (!element) {
      return;
    }
    const next = clampMenuPositionForElement({ left: origin.left, top: origin.top, element, viewportGap: 8 });
    setPosition(prev => {
      if (prev && prev.left === next.left && prev.top === next.top) {
        return prev;
      }
      return next;
    });
  };

  const handleRename = (): void => {
    onRename();
    onClose();
  };

  const handleDelete = (): void => {
    onDelete();
    onClose();
  };

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent): void => {
      if (event.key !== 'Escape') {
        return;
      }
      event.preventDefault();
      onClose();
    };
    document.addEventListener('keydown', handleKeyDown, true);
    return () => {
      document.removeEventListener('keydown', handleKeyDown, true);
    };
  }, [onClose]);

  if (mobileViewport) {
    return (
      <CommonMenu
        position={{ top: origin.top, left: origin.left }}
        onClose={onClose}
        items={[
          { key: 1, text: t('agent.contextMenu.rename'), onClick: handleRename },
          { key: 2, text: t('agent.contextMenu.delete'), danger: true, onClick: handleDelete },
        ]}
      />
    );
  }

  return (
    <Portal>
      <ClickAwayListener onClickAway={onClose}>
        <Box
          ref={attachMenu}
          sx={{ position: 'fixed', left: position ? position.left: origin.left, top: position ? position.top: origin.top, zIndex: 'var(--tm-z-overlay)', minWidth: MENU_MIN_WIDTH, py: 0.5, borderRadius: 1, bgcolor: 'var(--tm-glass-dialog-bg)', border: '1px solid var(--tm-border)', boxShadow: 'var(--tm-menu-shadow)', backdropFilter: 'var(--tm-glass-dialog-blur)', WebkitBackdropFilter: 'var(--tm-glass-dialog-blur)' }}
          onContextMenu={event => {
            event.preventDefault();
            event.stopPropagation();
          }}
        >
          <MenuList dense disablePadding>
            <MenuItem onClick={handleRename} sx={{ cursor: 'default' }}>
              {t('agent.contextMenu.rename')}
            </MenuItem>
            <MenuItem onClick={handleDelete} sx={{ cursor: 'default', color: 'var(--tm-status-danger)' }}>
              {t('agent.contextMenu.delete')}
            </MenuItem>
          </MenuList>
        </Box>
      </ClickAwayListener>
    </Portal>
  );
}
