import { Box, Dialog } from '@mui/material';
import { useEffect, useState } from 'react';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { isEmpty } from '@/common/util/StringUtil.ts';
import { Footer } from '@/page/home/forward/edit/frame/Footer';
import { Header } from '@/page/common/components/dialog/Header.tsx';
import { useSnippetService } from '@/common/context/useAppHooks.tsx';
import { resolveErrorText } from '@/common/model/resp/ResultModel.ts';
import { useBackDismiss } from '@/page/common/hooks/useBackDismiss.ts';
import { AlertDialog } from '@/common/components/dialog/AlertDialog.tsx';
import { EditField } from '@/page/common/components/input/EditField.tsx';
import { StepperField } from '@/page/common/components/input/StepperField.tsx';
import { resolveSheetDialogProps } from '@/page/common/util/scopedDialogUtil.ts';
import { TextareaField } from '@/page/common/components/input/TextareaField.tsx';
import { snippetVariables, verifySnippet, type Snippet } from '@/common/model/Snippet.ts';

export function EditDialog(props: { open: boolean; snippet: Snippet; onSaved: () => void; onClose: () => void }) {
  const { t, tf } = useI18n();
  const { open, onSaved, onClose } = props;
  const snippetService = useSnippetService();
  const [saving, setSaving] = useState<boolean>(false);
  const [value, setValue] = useState<Snippet>(props.snippet);
  const variables = snippetVariables(value.command);
  const [alertInfo, setAlertInfo] = useState<{ showAlert: boolean; message: string | null }>({ showAlert: false, message: null });

  const handleNameChange = (name: string): void => {
    setValue({ ...value, name });
  };

  const handleCommandChange = (command: string): void => {
    setValue({ ...value, command });
  };

  const handleRemarkChange = (remark: string): void => {
    setValue({ ...value, remark });
  };

  const handleSortChange = (sort: number): void => {
    setValue({ ...value, sort });
  };

  const handleSave = (): void => {
    if (!verifySnippet(value) || saving) {
      return;
    }
    setSaving(true);
    const action = isEmpty(value.id) ? snippetService.insert(value) : snippetService.update(value);
    action
      .then(() => {
        setSaving(false);
        onSaved();
      })
      .catch(err => {
        setSaving(false);
        setAlertInfo({ showAlert: true, message: tf('home.common.saveFailed', { error: resolveErrorText(err) }) });
      });
  };

  useBackDismiss(open, onClose, 'snippet-edit');

  useEffect(() => {
    setSaving(false);
    setValue(props.snippet);
  }, [props.snippet]);

  return (
    <Dialog open={open} onClose={onClose} {...resolveSheetDialogProps({ className: 'tm-edit-dialog-paper tm-snippet-dialog-paper' })}>
      <Box className="tm-edit-dialog-body">
        <Header title={t('home.snippet.editTitle')} onClose={onClose} />
        <Box sx={{ flex: 1, minHeight: 0, overflow: 'auto', px: 2, py: 2, display: 'flex', flexDirection: 'column', gap: 1.5 }}>
          <EditField label={t('home.common.name')} value={value.name} onChange={handleNameChange} placeholder={t('home.snippet.namePlaceholder')} />
          <TextareaField label={t('home.snippet.command')} value={value.command} onChange={handleCommandChange} placeholder={t('home.snippet.commandPlaceholder')} language="shell" minRows={1} maxRows={1} expandable autoFocus />
          <Box className="tm-snippet-variable-hint">{variables.length === 0 ? t('home.snippet.variableNone') : tf('home.snippet.variableList', { names: variables.join(', ') })}</Box>
          <EditField label={t('account.merge.fields.remark')} value={value.remark} onChange={handleRemarkChange} placeholder={t('home.snippet.remarkPlaceholder')} />
          <StepperField label={t('home.snippet.sort')} value={value.sort} onChange={handleSortChange} min={0} max={9999} />
        </Box>
        <AlertDialog duration={1500} open={alertInfo.showAlert} handleClose={() => setAlertInfo({ showAlert: false, message: null })} message={alertInfo.message} type="error" />
        <Footer disable={!verifySnippet(value) || saving} saving={saving} onCancel={onClose} onSave={handleSave} />
      </Box>
    </Dialog>
  );
}
