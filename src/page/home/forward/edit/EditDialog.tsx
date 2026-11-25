import { Footer } from './frame/Footer';
import { Box, Dialog } from '@mui/material';
import { useEffect, useState } from 'react';
import { TypeTabs } from './section/TypeTabs.tsx';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { isNil } from '@/common/util/StringUtil.ts';
import { RuleSection } from './section/RuleSection';
import type { PortForward } from '@/common/model/PortForward.ts';
import { verifyPortForward } from '@/common/model/PortForward.ts';
import { Header } from '@/page/common/components/dialog/Header.tsx';
import { resolveErrorText } from '@/common/model/resp/ResultModel.ts';
import { useBackDismiss } from '@/page/common/hooks/useBackDismiss.ts';
import { AlertDialog } from '@/common/components/dialog/AlertDialog.tsx';
import { resolveSheetDialogProps } from '@/page/common/util/scopedDialogUtil.ts';
import type { SelectOption } from '@/page/common/components/input/SelectField.tsx';
import { useDeviceService, usePortForwardService } from '@/common/context/useAppHooks.tsx';

export function EditDialog(props: { open: boolean; portForward: PortForward; onChange?: (next: PortForward) => void; onSave?: (value: PortForward) => void; onClose: () => void }) {
  const { t, tf } = useI18n();
  const deviceService = useDeviceService();
  const { open, onChange, onClose, onSave } = props;
  const portForwardService = usePortForwardService();
  const [verify, setVerify] = useState<boolean>(false);
  const [saving, setSaving] = useState<boolean>(false);
  const [value, setValue] = useState<PortForward>(props.portForward);
  const [deviceOptions, setDeviceOptions] = useState<SelectOption[]>([]);
  const [alertInfo, setAlertInfo] = useState<{ showAlert: boolean; message: string | null }>({ showAlert: false, message: null });

  const handleValueChange = (next: PortForward): void => {
    setVerify(verifyPortForward(next));
    setValue(next);
    onChange?.(next);
  };

  const handleKindChange = (kind: number): void => {
    handleValueChange({ ...value, kind });
  };

  const handleSave = (): void => {
    if (!verify || saving) {
      return;
    }
    setSaving(true);
    const action = isNil(value.id) ? portForwardService.insert(value) : portForwardService.update(value);
    action
      .then(saved => {
        setSaving(false);
        onSave?.(saved);
      })
      .catch(err => {
        setSaving(false);
        setAlertInfo({ showAlert: true, message: tf('home.common.saveFailed', { error: resolveErrorText(err) }) });
      });
  };

  useBackDismiss(open, onClose, 'forward-edit');

  useEffect(() => {
    setSaving(false);
    setValue(props.portForward);
    setVerify(verifyPortForward(props.portForward));
  }, [props.portForward]);

  useEffect(() => {
    return deviceService.addDeviceListChangeListener(list => {
      setDeviceOptions(list.map(item => ({ value: item.device.id, label: item.device.name })));
    });
  }, [deviceService]);

  useEffect(() => {
    if (!open) {
      return;
    }
    void deviceService.selectAllWithConfig();
  }, [open, deviceService]);

  return (
    <Dialog open={open} onClose={onClose} {...resolveSheetDialogProps({ className: 'tm-edit-dialog-paper' })}>
      <Box className="tm-edit-dialog-body">
        <Header title={t('home.forward.editTitle')} onClose={onClose} />
        <Box sx={{ flex: 1, minHeight: 0, overflow: 'auto', px: 2, py: 2, display: 'flex', flexDirection: 'column', gap: 1.5 }}>
          <TypeTabs value={value.kind} onChange={handleKindChange} />
          <RuleSection value={value} onChange={handleValueChange} deviceOptions={deviceOptions} />
        </Box>
        <AlertDialog duration={1500} open={alertInfo.showAlert} handleClose={() => setAlertInfo({ showAlert: false, message: null })} message={alertInfo.message} type="error" />
        <Footer disable={!verify || saving} saving={saving} onCancel={onClose} onSave={handleSave} />
      </Box>
    </Dialog>
  );
}
