import type { ReactNode } from 'react';
import { Box, Menu, MenuItem } from '@mui/material';
import { useEffect, useRef, useState } from 'react';
import { isNotEmpty } from '@/common/util/StringUtil.ts';
import { usePlatformMobile } from '@/common/context/usePlatformMobile';
import { useOpenTransition } from '@/page/common/hooks/useOpenTransition.ts';

export interface CommonMenuPosition {
  top: number;
  left: number;
}

export interface CommonMenuItem {
  key: number;
  text: string;
  icon?: ReactNode;
  shortcut?: string;
  accent?: boolean;
  danger?: boolean;
  disabled?: boolean;
  onClick: () => void;
}

export function CommonMenu(props: { position: CommonMenuPosition | null; items: CommonMenuItem[]; onClose: () => void }) {
  const { position, items, onClose } = props;
  const open = position !== null;
  const mobileViewport = usePlatformMobile();
  const { mounted } = useOpenTransition(open, 180);
  const paperRef = useRef<HTMLDivElement | null>(null);
  const [anchor, setAnchor] = useState<CommonMenuPosition>({ top: 0, left: 0 });

  const buildItemClassName = (item: CommonMenuItem): string => {
    if (item.danger === true) {
      return 'tm-menu-item tm-menu-item-danger';
    }
    if (item.accent === true) {
      return 'tm-menu-item is-accent';
    }
    return 'tm-menu-item';
  };

  const handleClickItem = (item: CommonMenuItem): void => {
    if (item.disabled === true) {
      return;
    }
    item.onClick();
    onClose();
  };

  useEffect(() => {
    if (position) {
      setAnchor(position);
    }
  }, [position]);

  useEffect(() => {
    if (!open) {
      return;
    }
    const closeIfOutside = (event: Event): void => {
      const target = event.target;
      if (!(target instanceof Node)) {
        return;
      }
      if (paperRef.current?.contains(target)) {
        return;
      }
      onClose();
    };
    document.addEventListener('pointerdown', closeIfOutside, true);
    return () => {
      document.removeEventListener('pointerdown', closeIfOutside, true);
    };
  }, [open, onClose]);

  const itemNodes = items.map(item => (
    <MenuItem key={item.key} className={buildItemClassName(item)} disabled={item.disabled === true} onClick={() => handleClickItem(item)}>
      {!mobileViewport && item.icon ? (
        <Box component="span" className="tm-menu-item-icon">
          {item.icon}
        </Box>
      ) : null}
      <Box component="span" className="tm-menu-item-text">
        {item.text}
      </Box>
      {!mobileViewport && isNotEmpty(item.shortcut) ? (
        <Box component="span" className="tm-menu-item-shortcut">
          {item.shortcut}
        </Box>
      ) : null}
    </MenuItem>
  ));

  if (!mounted) {
    return null;
  }

  return (
    <Menu
      open={open}
      hideBackdrop
      disableScrollLock
      anchorReference="anchorPosition"
      anchorPosition={anchor}
      transformOrigin={mobileViewport ? {vertical: 'top', horizontal: 'right'} : {vertical: 'top', horizontal: 'left'}}
      onClose={onClose}
      slotProps={{
        root: { sx: { pointerEvents: 'none' } },
        paper: { className: 'tm-menu-paper', sx: { pointerEvents: 'auto' }, ref: paperRef },
        list: { className: 'tm-menu-list' },
      }}
    >
      {itemNodes}
    </Menu>
  );
}
