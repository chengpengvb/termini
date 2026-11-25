import { useEffect, useState } from 'react';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { Box, Dialog, Typography } from '@mui/material';
import { Footer } from '@/page/home/forward/edit/frame/Footer';
import { Header } from '@/page/common/components/dialog/Header.tsx';
import { useAccountService } from '@/common/context/useAppHooks.tsx';
import { useBackDismiss } from '@/page/common/hooks/useBackDismiss.ts';
import { AlertDialog } from '@/common/components/dialog/AlertDialog.tsx';
import { resolveSheetDialogProps } from '@/page/common/util/scopedDialogUtil.ts';
import { TextareaField } from '@/page/common/components/input/TextareaField.tsx';

const CONTENT_MAX = 2048;

export function FeedbackDialog(props: { open: boolean; onSent: () => void; onClose: () => void }) {
  const { t } = useI18n();
  const { open, onSent, onClose } = props;
  const accountService = useAccountService();
  const [content, setContent] = useState<string>('');
  const [sending, setSending] = useState<boolean>(false);
  const [alertInfo, setAlertInfo] = useState<{ showAlert: boolean; message: string | null }>({ showAlert: false, message: null });
  const trimmed = content.trim();

  const handleClose = (): void => {
    if (sending) {
      return;
    }
    onClose();
  };

  const handleSend = (): void => {
    if (trimmed === '' || sending) {
      return;
    }
    const cooldownLeftMs = accountService.feedbackCooldownLeftMs();
    if (cooldownLeftMs > 0) {
      const minutes = Math.ceil(cooldownLeftMs / 60000);
      setAlertInfo({ showAlert: true, message: t('set.about.feedbackTooOften').replace('{minutes}', String(minutes)) });
      return;
    }
    setSending(true);
    accountService
      .submitFeedback(trimmed)
      .then(() => {
        setSending(false);
        onSent();
      })
      .catch(() => {
        setSending(false);
        setAlertInfo({ showAlert: true, message: t('set.about.feedbackFailed') });
      });
  };

  useBackDismiss(open, handleClose, 'feedback-edit');

  useEffect(() => {
    if (!open) {
      return;
    }
    setContent('');
    setSending(false);
  }, [open]);

  return (
    <Dialog open={open} onClose={handleClose} {...resolveSheetDialogProps({ className: 'tm-edit-dialog-paper tm-feedback-dialog-paper' })}>
      <Box className="tm-edit-dialog-body">
        <Header title={t('set.about.feedback')} onClose={handleClose} />
        <Box sx={{ flex: 1, minHeight: 0, overflow: 'auto', px: 2, py: 2, display: 'flex', flexDirection: 'column', gap: 1 }}>
          <TextareaField value={content} onChange={setContent} placeholder={t('set.about.feedbackPlaceholder')} minRows={6} maxRows={12} maxLength={CONTENT_MAX} />
          <Typography className="tm-set-row-hint" sx={{ textAlign: 'right' }}>
            {content.length} / {CONTENT_MAX}
          </Typography>
        </Box>
        <AlertDialog duration={2500} open={alertInfo.showAlert} handleClose={() => setAlertInfo({ showAlert: false, message: null })} message={alertInfo.message} type="error" />
        <Footer disable={trimmed === '' || sending} saving={sending} saveLabel={t('set.about.feedbackSubmit')} onCancel={handleClose} onSave={handleSend} />
      </Box>
    </Dialog>
  );
}
