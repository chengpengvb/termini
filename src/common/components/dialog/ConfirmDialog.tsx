import { useMemo, useRef } from 'react';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { isNotNil } from '@/common/util/StringUtil.ts';
import type { MouseEvent, ReactNode, RefObject } from 'react';
import { useBackDismiss } from '@/page/common/hooks/useBackDismiss.ts';
import { Box, Button, Dialog, Tooltip, Typography } from '@mui/material';
import { SheetTransition } from '@/page/common/components/dialog/SheetTransition.tsx';
import { OverlayPaper, resolveScopedDialogProps } from '@/page/common/util/scopedDialogUtil.ts';

interface ConfirmDialogProps {
  open: boolean;
  title?: string;
  message: string;
  danger?: boolean;
  children?: ReactNode;
  cancelLabel?: string;
  confirmLabel?: string;
  onCancel: () => void;
  onConfirm: () => void;
  error?: string | null;
  confirmDisabled?: boolean;
  autoFocus?: boolean;
  dialogContainerRef?: RefObject<HTMLElement | null>;
}

export function ConfirmDialog(props: ConfirmDialogProps) {
  const { open, title, message, error, cancelLabel, confirmLabel, confirmDisabled, autoFocus = false, danger, children, dialogContainerRef, onCancel, onConfirm } = props;
  const { t } = useI18n();
  const dialogContainer = dialogContainerRef ? dialogContainerRef.current : null;
  const backdropPointerDownRef = useRef(false);

  const resolveCancelLabel = (): string => {
    if (isNotNil(cancelLabel)) {
      return cancelLabel;
    }
    return t('common.cancel');
  };

  const resolveConfirmLabel = (): string => {
    if (isNotNil(confirmLabel)) {
      return confirmLabel;
    }
    return t('common.confirm');
  };

  function handlePaperMouseDown(event: MouseEvent): void {
    event.stopPropagation();
  }

  function handleTransitionEnter(node: HTMLElement): void {
    const input = node.querySelector('input');
    if (!(input instanceof HTMLInputElement)) {
      return;
    }
    input.focus({ preventScroll: true });
  }

  function handleBackdropPointerDown(): void {
    backdropPointerDownRef.current = true;
  }

  function handleCancel(): void {
    backdropPointerDownRef.current = false;
    onCancel();
  }

  function handleClose(_event: object, reason: 'backdropClick' | 'escapeKeyDown'): void {
    if (reason === 'backdropClick' && !backdropPointerDownRef.current) {
      return;
    }
    backdropPointerDownRef.current = false;
    onCancel();
  }

  useBackDismiss(open, handleCancel, 'confirm-dialog');

  const confirmClassName = danger === true ? 'tm-btn tm-btn-danger' : 'tm-btn tm-btn-primary';

  const scopedDialogProps = useMemo(() => {
    return resolveScopedDialogProps(dialogContainer);
  }, [dialogContainer]);

  return (
    <Dialog
      open={open}
      onClose={handleClose}
      container={scopedDialogProps.container}
      disablePortal={scopedDialogProps.disablePortal}
      disableScrollLock
      disableAutoFocus={!autoFocus}
      disableRestoreFocus={!autoFocus}
      disableEnforceFocus={autoFocus ? false : scopedDialogProps.disableEnforceFocus}
      className={scopedDialogProps.className}
      sx={scopedDialogProps.sx}
      slots={{ paper: OverlayPaper, transition: SheetTransition }}
      slotProps={{
        backdrop: { ...scopedDialogProps.backdropSlotProps, onPointerDown: handleBackdropPointerDown },
        paper: { className: 'tm-dialog-paper', sx: { width: 380 }, onMouseDown: handlePaperMouseDown },
        transition: { onEnter: handleTransitionEnter },
      }}
    >
      {isNotNil(title) ? (
        <Box sx={{ px: 2.5, pt: 2.25, pb: 0.5 }}>
          <Typography sx={{ fontSize: 15, fontWeight: 700, color: 'var(--tm-text-primary)' }}>{title}</Typography>
        </Box>
      ) : null}
      <Box sx={{ px: 2.5, pt: isNotNil(title) ? 1 : 2.25, pb: 1.5, minWidth: 0 }}>
        <Tooltip title={message} enterDelay={400}>
          <Typography sx={{ display: '-webkit-box', maxWidth: '100%', minWidth: 0, fontSize: 13, color: 'var(--tm-text-secondary)', overflow: 'clip', overflowWrap: 'anywhere', WebkitBoxOrient: 'vertical', WebkitLineClamp: 2 }}>{message}</Typography>
        </Tooltip>
        {children}
        {error != null ? (
          <Typography className="tm-confirm-dialog-error" title={error} sx={{ mt: 1.5 }}>
            {error}
          </Typography>
        ) : null}
      </Box>
      <Box className="tm-edit-dialog-footer">
        <Box className="tm-edit-dialog-footer-spacer" />
        <Button disableRipple onClick={handleCancel} className="tm-btn tm-btn-secondary">
          {resolveCancelLabel()}
        </Button>
        <Button disableRipple variant="contained" disabled={confirmDisabled === true} onClick={onConfirm} className={confirmClassName}>
          {resolveConfirmLabel()}
        </Button>
      </Box>
    </Dialog>
  );
}
