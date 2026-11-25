import { Footer } from './frame/Footer';
import { Box, Dialog } from '@mui/material';
import { useEffect, useState } from 'react';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { isNil } from '@/common/util/StringUtil.ts';
import { GeneralSection } from './section/GeneralSection';
import { KeyMaterialSection } from './section/KeyMaterialSection';
import { Header } from '@/page/common/components/dialog/Header.tsx';
import type { ConfigKeyPair } from '@/common/model/ConfigKeyPair.ts';
import { resolveErrorText } from '@/common/model/resp/ResultModel.ts';
import { verifyConfigKeyPair } from '@/common/model/ConfigKeyPair.ts';
import { useBackDismiss } from '@/page/common/hooks/useBackDismiss.ts';
import { AlertDialog } from '@/common/components/dialog/AlertDialog.tsx';
import { useConfigKeyPairService } from '@/common/context/useAppHooks.tsx';
import { resolveSheetDialogProps } from '@/page/common/util/scopedDialogUtil.ts';

export function EditDialog(props: { open: boolean; configKeyPair: ConfigKeyPair; onChange?: (next: ConfigKeyPair) => void; onSave?: (value: ConfigKeyPair) => void; onClose: () => void }) {
  const { t, tf } = useI18n();
  const { open, onChange, onClose, onSave } = props;
  const configKeyPairService = useConfigKeyPairService();
  const [verify, setVerify] = useState<boolean>(false);
  const [value, setValue] = useState<ConfigKeyPair>(props.configKeyPair);
  const [alertInfo, setAlertInfo] = useState<{ showAlert: boolean; message: string | null }>({ showAlert: false, message: null });

  const holdValueChange = (next: ConfigKeyPair): void => {
    setVerify(verifyConfigKeyPair(next));
    setValue(next);
    onChange?.(next);
  };

  const handleValueChange = (next: ConfigKeyPair): void => {
    holdValueChange(next);
  };

  const handleSave = (): void => {
    if (!verify) {
      return;
    }
    const action = isNil(value.id) ? configKeyPairService.insert(value) : configKeyPairService.update(value);
    action
      .then(saved => {
        onSave?.(saved);
      })
      .catch(err => {
        setAlertInfo({ showAlert: true, message: tf('home.common.saveFailed', { error: resolveErrorText(err) }) });
      });
  };

  useBackDismiss(open, onClose, 'key-edit');

  useEffect(() => {
    setValue(props.configKeyPair);
    setVerify(verifyConfigKeyPair(props.configKeyPair));
  }, [props.configKeyPair]);

  return (
    <Dialog open={open} onClose={onClose} {...resolveSheetDialogProps({ className: 'tm-edit-dialog-paper' })}>
      <Box className="tm-edit-dialog-body">
        <Header title={t('home.keys.editTitle')} onClose={onClose} />
        <Box sx={{ flex: 1, minHeight: 0, overflow: 'auto', px: 2, py: 2, display: 'flex', flexDirection: 'column', gap: 2.5 }}>
          <GeneralSection value={value} onChange={handleValueChange} />
          <KeyMaterialSection value={value} onChange={handleValueChange} />
        </Box>
        <AlertDialog duration={1500} open={alertInfo.showAlert} handleClose={() => setAlertInfo({ showAlert: false, message: null })} message={alertInfo.message} type="error" />
        <Footer disable={!verify} onCancel={onClose} onSave={handleSave} />
      </Box>
    </Dialog>
  );
}
