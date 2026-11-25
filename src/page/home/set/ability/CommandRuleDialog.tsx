import { useI18n } from '@/common/i18n/useI18n.ts';
import { useEffect, useMemo, useState } from 'react';
import { Box, Dialog, Typography } from '@mui/material';
import { RULE_DECISION } from '@/common/model/CommandRule.ts';
import { Footer } from '@/page/home/forward/edit/frame/Footer';
import { Header } from '@/page/common/components/dialog/Header.tsx';
import { resolveErrorText } from '@/common/model/resp/ResultModel.ts';
import { useBackDismiss } from '@/page/common/hooks/useBackDismiss.ts';
import { AlertDialog } from '@/common/components/dialog/AlertDialog.tsx';
import { EditField } from '@/page/common/components/input/EditField.tsx';
import { useCommandRuleService } from '@/common/context/useAppHooks.tsx';
import { SelectField } from '@/page/common/components/input/SelectField.tsx';
import { resolveSheetDialogProps } from '@/page/common/util/scopedDialogUtil.ts';
import type { SelectOption } from '@/page/common/components/input/SelectField.tsx';

export function CommandRuleDialog(props: { open: boolean; onSaved: () => void; onClose: () => void }) {
  const { t, tf } = useI18n();
  const { open, onSaved, onClose } = props;
  const commandRuleService = useCommandRuleService();
  const [saving, setSaving] = useState<boolean>(false);
  const [pattern, setPattern] = useState<string>('');
  const [justification, setJustification] = useState<string>('');
  const [decision, setDecision] = useState<number>(RULE_DECISION.PROMPT);
  const [alertInfo, setAlertInfo] = useState<{ showAlert: boolean; message: string | null }>({ showAlert: false, message: null });

  const handleSave = (): void => {
    if (pattern.trim() === '' || saving) {
      return;
    }
    setSaving(true);
    commandRuleService
      .create({ pattern: pattern.trim(), decision, deviceId: null, justification: justification.trim() === '' ? null : justification.trim() })
      .then(() => {
        setSaving(false);
        onSaved();
      })
      .catch(err => {
        setSaving(false);
        setAlertInfo({ showAlert: true, message: tf('home.common.saveFailed', { error: resolveErrorText(err) }) });
      });
  };

  const decisionOptions = useMemo<SelectOption[]>(
    () => [
      { value: String(RULE_DECISION.ALLOW), label: t('set.commandRule.allow') },
      { value: String(RULE_DECISION.PROMPT), label: t('set.commandRule.prompt') },
      { value: String(RULE_DECISION.FORBIDDEN), label: t('set.commandRule.forbidden') },
    ],
    [t],
  );

  useBackDismiss(open, onClose, 'command-rule-edit');

  useEffect(() => {
    if (!open) {
      return;
    }
    setSaving(false);
    setPattern('');
    setJustification('');
    setDecision(RULE_DECISION.PROMPT);
  }, [open]);

  return (
    <Dialog open={open} onClose={onClose} {...resolveSheetDialogProps({ className: 'tm-edit-dialog-paper tm-rule-dialog-paper' })}>
      <Box className="tm-edit-dialog-body">
        <Header title={t('set.commandRule.addTitle')} onClose={onClose} />
        <Box sx={{ flex: 1, minHeight: 0, overflow: 'auto', px: 2, py: 2, display: 'flex', flexDirection: 'column', gap: 1.5 }}>
          <EditField label={t('set.commandRule.pattern')} labelWidth={72} value={pattern} onChange={setPattern} placeholder={t('set.commandRule.patternPlaceholder')} autoFocus />
          <SelectField label={t('set.commandRule.decision')} labelWidth={72} value={String(decision)} onChange={value => setDecision(Number(value))} options={decisionOptions} />
          <EditField label={t('set.commandRule.justification')} labelWidth={72} value={justification} onChange={setJustification} placeholder={t('set.commandRule.justificationPlaceholder')} />
          <Typography className="tm-set-row-hint">{t('set.commandRule.patternHint')}</Typography>
        </Box>
        <AlertDialog duration={1500} open={alertInfo.showAlert} handleClose={() => setAlertInfo({ showAlert: false, message: null })} message={alertInfo.message} type="error" />
        <Footer disable={pattern.trim() === '' || saving} saving={saving} onCancel={onClose} onSave={handleSave} />
      </Box>
    </Dialog>
  );
}
