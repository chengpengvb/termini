import { useState } from 'react';
import { Box } from '@mui/material';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { isEmpty } from '@/common/util/StringUtil.ts';
import { useAccountService } from '@/common/context/useAppHooks.tsx';
import { resolveErrorText } from '@/common/model/resp/ResultModel.ts';
import { EditField } from '@/page/common/components/input/EditField.tsx';
import { ConfirmDialog } from '@/common/components/dialog/ConfirmDialog.tsx';

interface AccountDeleteDialogProps {
  open: boolean;
  email: string;
  onClose: () => void;
}

export function AccountDeleteDialog(props: AccountDeleteDialogProps) {
  const { t, tf } = useI18n();
  const [code, setCode] = useState('');
  const { open, email, onClose } = props;
  const accountService = useAccountService();
  const [sending, setSending] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [codeSent, setCodeSent] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const resolveSendLabel = (): string => {
    if (sending) {
      return t('account.delete.sending');
    }
    if (codeSent) {
      return t('account.delete.resend');
    }
    return t('account.delete.sendCode');
  };

  const handleCodeChange = (value: string): void => {
    setError(null);
    setCode(value);
  };

  const handleSendCode = (): void => {
    if (sending || deleting) {
      return;
    }
    setSending(true);
    setError(null);
    accountService
      .sendDeleteCode()
      .then(() => {
        setCodeSent(true);
      })
      .catch(e => {
        setError(resolveErrorText(e));
      })
      .finally(() => {
        setSending(false);
      });
  };

  const handleCancel = (): void => {
    if (deleting) {
      return;
    }
    setCode('');
    setError(null);
    setCodeSent(false);
    onClose();
  };

  const handleConfirm = (): void => {
    if (deleting) {
      return;
    }
    if (isEmpty(code)) {
      setError(t('account.delete.codeRequired'));
      return;
    }
    setDeleting(true);
    accountService
      .deleteAccount(code.trim())
      .then(() => {
        setCode('');
        setCodeSent(false);
        onClose();
      })
      .catch(e => {
        setError(resolveErrorText(e));
      })
      .finally(() => {
        setDeleting(false);
      });
  };

  return (
    <ConfirmDialog danger open={open} error={error} title={t('account.delete.title')} message={tf('account.delete.message', { email })} confirmLabel={deleting ? t('account.delete.deleting') : t('account.delete.confirm')} confirmDisabled={deleting} onCancel={handleCancel} onConfirm={handleConfirm}>
      <Box className={sending ? 'tm-account-login-code is-wait' : 'tm-account-login-code'} sx={{ mt: 1.5 }}>
        <EditField autoFocus value={code} onChange={handleCodeChange} placeholder={t('account.delete.codePlaceholder')} actionLabel={t('account.delete.sendCode')} actionIcon={resolveSendLabel()} onAction={handleSendCode} />
      </Box>
    </ConfirmDialog>
  );
}
