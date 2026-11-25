import { useEffect, useState } from 'react';
import { DeviceList } from './DeviceList.tsx';
import { useI18n } from '@/common/i18n/useI18n.ts';
import type { Device } from '@/common/model/Device.ts';
import type { JobDevice } from '@/common/model/JobDevice.ts';
import { Box, Button, Dialog, Typography } from '@mui/material';
import { Header } from '@/page/common/components/dialog/Header.tsx';
import { useBackDismiss } from '@/page/common/hooks/useBackDismiss.ts';
import { resolveSheetDialogProps } from '@/page/common/util/scopedDialogUtil.ts';
import { useDeviceService, useJobDeviceService } from '@/common/context/useAppHooks.tsx';

interface AddDialogProps {
  open: boolean;
  onClose: () => void;
  onAdded: () => void;
}

export function AddDialog(props: AddDialogProps) {
  const { t, tf } = useI18n();
  const { open, onClose, onAdded } = props;
  const deviceService = useDeviceService();
  const [saving, setSaving] = useState(false);
  const jobDeviceService = useJobDeviceService();
  const [devices, setDevices] = useState<Device[]>([]);
  const [selectedIds, setSelectedIds] = useState<string[]>([]);
  const [jobDevices, setJobDevices] = useState<JobDevice[]>([]);
  const taken = new Set(jobDevices.map(item => item.device_id));
  const candidates = devices.filter(device => !taken.has(device.id));

  const toggleOne = (id: string): void => {
    setSelectedIds(prev => (prev.includes(id) ? prev.filter(item => item !== id) : [...prev, id]));
  };

  const handleConfirm = (): void => {
    if (selectedIds.length === 0 || saving) {
      return;
    }
    setSaving(true);
    void jobDeviceService
      .saveBatch(selectedIds)
      .then(() => {
        onClose();
        onAdded();
      })
      .catch(err => {
        console.error('[job] save_batch error', err);
      })
      .finally(() => {
        setSaving(false);
      });
  };

  useBackDismiss(open, onClose, 'job-add-dialog');

  useEffect(() => {
    return deviceService.addDeviceListChangeListener(list => {
      setDevices(list.map(item => item.device));
    });
  }, [deviceService]);

  useEffect(() => {
    if (!open) {
      return;
    }
    setSelectedIds([]);
    void deviceService.selectAllWithConfig();
    void jobDeviceService.selectAllDetail().then(setJobDevices);
  }, [open, deviceService, jobDeviceService]);

  return (
    <Dialog open={open} onClose={onClose} {...resolveSheetDialogProps({ className: 'tm-edit-dialog-paper' })}>
      <Box className="tm-edit-dialog-body">
        <Header title={t('home.job.addDialogTitle')} onClose={onClose} />
        <Box className="tm-job-dialog-body tm-job-dialog-body-table">
          {candidates.length === 0 ? <Typography className="tm-job-muted">{t('home.job.addDialogHint')}</Typography> : <DeviceList candidates={candidates} selectedIds={selectedIds} onToggle={toggleOne} onChangeSelectedIds={setSelectedIds} />}
        </Box>
        <Box className="tm-edit-dialog-footer">
          <Button disableRipple className="tm-btn tm-btn-secondary" onClick={onClose}>
            {t('common.cancel')}
          </Button>
          <Box className="tm-edit-dialog-footer-spacer" />
          <Button disableRipple className="tm-btn tm-btn-primary" disabled={selectedIds.length === 0 || saving} onClick={handleConfirm}>
            {saving ? t('home.job.addSaving') : tf('home.job.addConfirmCount', { n: selectedIds.length })}
          </Button>
        </Box>
      </Box>
    </Dialog>
  );
}
