import { Box, Portal, Typography } from '@mui/material';
import { useLayoutEffect, useState, type ReactNode } from 'react';
import { useBackDismiss } from '@/page/common/hooks/useBackDismiss.ts';
import { useOpenTransition } from '@/page/common/hooks/useOpenTransition.ts';
import { useSheetDragDismiss } from '@/page/common/hooks/useSheetDragDismiss.ts';

interface ActionSheetProps {
  open: boolean;
  title?: string;
  onClose: () => void;
  children: ReactNode;
}

let heldPointers = 0;

function resolveRootClassName(active: boolean, pointerGuard: boolean): string {
  if (active && pointerGuard) {
    return 'tm-action-sheet-root is-open is-pointer-guard';
  }
  if (active) {
    return 'tm-action-sheet-root is-open';
  }
  if (pointerGuard) {
    return 'tm-action-sheet-root is-pointer-guard';
  }
  return 'tm-action-sheet-root';
}

function onHeldPointerDown(): void {
  heldPointers += 1;
}

function onHeldPointerUp(): void {
  if (heldPointers === 0) {
    return;
  }
  heldPointers -= 1;
}

document.addEventListener('pointerdown', onHeldPointerDown, true);
document.addEventListener('pointerup', onHeldPointerUp, true);
document.addEventListener('pointercancel', onHeldPointerUp, true);

export function ActionSheet(props: ActionSheetProps) {
  const { open, title, onClose, children } = props;
  const { mounted, active } = useOpenTransition(open, 320);
  const [pointerGuard, setPointerGuard] = useState(() => open && heldPointers > 0);
  const { panelStyle, bindPanel, bindHandle } = useSheetDragDismiss(onClose, active);

  useBackDismiss(open, onClose, 'action-sheet');

  useLayoutEffect(() => {
    if (!mounted || !open) {
      setPointerGuard(false);
      return;
    }
    if (heldPointers === 0) {
      setPointerGuard(false);
      return;
    }
    setPointerGuard(true);
    const release = (): void => {
      setPointerGuard(false);
      document.removeEventListener('pointerup', release, true);
      document.removeEventListener('pointercancel', release, true);
    };
    document.addEventListener('pointerup', release, true);
    document.addEventListener('pointercancel', release, true);
    return () => {
      document.removeEventListener('pointerup', release, true);
      document.removeEventListener('pointercancel', release, true);
    };
  }, [mounted, open]);

  if (!mounted) {
    return null;
  }

  return (
    <Portal>
      <Box
        className={resolveRootClassName(active, pointerGuard)}
        onContextMenuCapture={event => {
          event.preventDefault();
          event.stopPropagation();
        }}
      >
        <Box className="tm-action-sheet-backdrop" onPointerDown={onClose} />
        <Box className="tm-action-sheet-panel" data-tm-overlay="true" style={panelStyle} {...bindPanel}>
          <Box className="tm-action-sheet-handle" {...bindHandle} />
          {title ? <Typography className="tm-action-sheet-title">{title}</Typography> : null}
          {children}
        </Box>
      </Box>
    </Portal>
  );
}
