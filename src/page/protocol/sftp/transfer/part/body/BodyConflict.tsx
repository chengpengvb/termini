import type { RefObject } from 'react';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { Box, Button, Dialog, Typography } from '@mui/material';
import { SFTP_TRANSFER_ANSWER } from '@/common/model/req/event/WsSftp.ts';
import { RespSftpTransferConflict } from '@/common/model/resp/RespSftp.ts';
import { OverlayPaper, resolveScopedDialogProps } from '@/page/common/util/scopedDialogUtil.ts';

interface BodyConflictProps {
  conflict: RespSftpTransferConflict;
  dialogContainerRef?: RefObject<HTMLElement | null>;
  onAnswer: (code: number, requestId: string) => void;
}

export function BodyConflict(props: BodyConflictProps) {
  const { t } = useI18n();
  const { conflict, dialogContainerRef, onAnswer } = props;
  const scopedDialogProps = resolveScopedDialogProps(dialogContainerRef ? dialogContainerRef.current : null);

  const answer = (code: number): void => {
    onAnswer(code, conflict.requestId);
  };

  const resolveDangerSx = () => {
    return { fontSize: 13, bgcolor: 'var(--tm-bg-danger)', color: 'var(--tm-text-on-danger)', '&:hover': { bgcolor: 'var(--tm-bg-danger)', filter: 'brightness(1.08)' } };
  };

  const resolvePrimarySx = () => {
    return { fontSize: 13, bgcolor: 'var(--tm-bg-selected)', color: 'var(--tm-text-primary)', '&:hover': { bgcolor: 'var(--tm-bg-selected)', filter: 'brightness(1.08)' } };
  };

  return (
    <Dialog
      open
      onClose={() => answer(SFTP_TRANSFER_ANSWER.SKIP)}
      container={scopedDialogProps.container}
      disablePortal={scopedDialogProps.disablePortal}
      disableScrollLock={scopedDialogProps.disableScrollLock}
      disableAutoFocus={scopedDialogProps.disableAutoFocus}
      disableRestoreFocus={scopedDialogProps.disableRestoreFocus}
      disableEnforceFocus={scopedDialogProps.disableEnforceFocus}
      className={scopedDialogProps.className}
      sx={scopedDialogProps.sx}
      slots={{ paper: OverlayPaper }}
      slotProps={{ backdrop: scopedDialogProps.backdropSlotProps, paper: { className: 'tm-dialog-paper', sx: { width: 'min(100vw - 32px, 460px)', maxWidth: 'calc(100vw - 32px)' } } }}
    >
      <Box sx={{ px: 2.5, pt: 2.25, pb: 0.5 }}>
        <Typography sx={{ fontSize: 15, fontWeight: 700, color: 'var(--tm-text-primary)' }}>{t('protocol.sftp.conflictTitle')}</Typography>
      </Box>
      <Box sx={{ px: 2.5, pt: 1, pb: 1.5 }}>
        <Typography sx={{ fontSize: 13, color: 'var(--tm-text-secondary)', mb: 1 }}>{t('protocol.sftp.conflictMessage')}</Typography>
        <Typography sx={{ fontSize: 13, color: 'var(--tm-text-primary)', wordBreak: 'break-all', bgcolor: 'var(--tm-bg-elevated)', p: 1, borderRadius: 'var(--tm-dialog-radius)' }}>{conflict.path}</Typography>
      </Box>
      <Box sx={{ px: 2, py: 1.25, display: 'flex', justifyContent: 'flex-end', gap: 1, flexWrap: 'wrap', borderTop: '1px solid var(--tm-border-subtle)' }}>
        <Button onClick={() => answer(SFTP_TRANSFER_ANSWER.SKIP_ALL)} sx={{ fontSize: 13, color: 'var(--tm-text-secondary)' }}>
          {t('protocol.sftp.skipAll')}
        </Button>
        <Button onClick={() => answer(SFTP_TRANSFER_ANSWER.SKIP)} sx={{ fontSize: 13, color: 'var(--tm-text-secondary)' }}>
          {t('protocol.sftp.skip')}
        </Button>
        <Button variant="contained" onClick={() => answer(SFTP_TRANSFER_ANSWER.OVERWRITE_ALL)} sx={resolvePrimarySx()}>
          {t('protocol.sftp.overwriteAll')}
        </Button>
        <Button variant="contained" onClick={() => answer(SFTP_TRANSFER_ANSWER.OVERWRITE)} sx={resolveDangerSx()}>
          {t('protocol.sftp.overwrite')}
        </Button>
      </Box>
    </Dialog>
  );
}
