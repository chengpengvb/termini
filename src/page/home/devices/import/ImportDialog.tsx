import { useEffect, useState } from 'react';
import { ImportList } from './ImportList.tsx';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { Header } from '@/page/common/components/dialog/Header.tsx';
import { resolveErrorText } from '@/common/model/resp/ResultModel.ts';
import { useBackDismiss } from '@/page/common/hooks/useBackDismiss.ts';
import type { DeviceImportItem } from '@/common/model/DeviceImport.ts';
import { useDeviceImportService } from '@/common/context/useAppHooks.tsx';
import { Box, Button, Dialog, InputBase, Typography } from '@mui/material';
import { itemKey, matchItem } from '@/page/common/util/deviceImportUtil.ts';
import { DEVICE_IMPORT_PASTE_SOURCES } from '@/common/model/DeviceImport.ts';
import { resolveSheetDialogProps } from '@/page/common/util/scopedDialogUtil.ts';

interface ImportDialogProps {
  open: boolean;
  source: number;
  onClose: () => void;
  onImported: () => void;
}

export function ImportDialog(props: ImportDialogProps) {
  const { t, tf } = useI18n();
  const [error, setError] = useState('');
  const [content, setContent] = useState('');
  const [summary, setSummary] = useState('');
  const [saving, setSaving] = useState(false);
  const [scanned, setScanned] = useState(false);
  const [scanning, setScanning] = useState(false);
  const { open, source, onClose, onImported } = props;
  const deviceImportService = useDeviceImportService();
  const [query, setQuery] = useState('');
  const [items, setItems] = useState<DeviceImportItem[]>([]);
  const [selectedKeys, setSelectedKeys] = useState<string[]>([]);
  const pasteable = DEVICE_IMPORT_PASTE_SOURCES.includes(source);
  const keyword = query.trim().toLowerCase();
  const filtered = keyword ? items.filter(item => matchItem(item, keyword)) : items;
  const selectedVisibleKeys = filtered
    .filter(item => !item.exists)
    .map(itemKey)
    .filter(key => selectedKeys.includes(key));

  const toggleOne = (key: string): void => {
    setSelectedKeys(prev => (prev.includes(key) ? prev.filter(item => item !== key) : [...prev, key]));
  };

  const handleScan = (): void => {
    if (scanning) {
      return;
    }
    setError('');
    setQuery('');
    setSummary('');
    setScanning(true);
    void deviceImportService
      .preview(source, content)
      .then(list => {
        setItems(list);
        setScanned(true);
        setSelectedKeys(list.filter(item => !item.exists).map(itemKey));
      })
      .catch(err => {
        setItems([]);
        setScanned(false);
        setSelectedKeys([]);
        setError(resolveErrorText(err));
      })
      .finally(() => {
        setScanning(false);
      });
  };

  const handleConfirm = (): void => {
    if (selectedVisibleKeys.length === 0 || saving) {
      return;
    }
    const picked = items.filter(item => selectedVisibleKeys.includes(itemKey(item)));
    setError('');
    setSaving(true);
    void deviceImportService
      .apply(picked)
      .then(result => {
        setItems([]);
        setQuery('');
        setSelectedKeys([]);
        setScanned(false);
        setSummary(tf('home.devices.importResult', { imported: result.imported, skipped: result.skipped, jump: result.jump_linked }));
        onImported();
      })
      .catch(err => {
        setError(resolveErrorText(err));
      })
      .finally(() => {
        setSaving(false);
      });
  };

  useBackDismiss(open, onClose, 'device-import-dialog');

  useEffect(() => {
    if (!open) {
      return;
    }
    setError('');
    setItems([]);
    setContent('');
    setSummary('');
    setQuery('');
    setScanned(false);
    setSelectedKeys([]);
  }, [open]);

  return (
    <Dialog open={open} onClose={onClose} {...resolveSheetDialogProps({ className: 'tm-edit-dialog-paper' })}>
      <Box className="tm-edit-dialog-body">
        <Header title={`${t('home.devices.importTitle')} · ${t(`home.devices.importSource.${source}`)}`} onClose={onClose} />
        <Box className="tm-job-dialog-body tm-job-dialog-body-table">
          <Typography className="tm-job-muted">{t(`home.devices.importHint.${source}`)}</Typography>
          {pasteable ? <InputBase multiline rows={3} className="tm-home-import-paste" placeholder={t(`home.devices.importPastePlaceholder.${source}`)} value={content} onChange={event => setContent(event.target.value)} /> : null}
          <Box className="tm-home-import-scan-row">
            <Button disableRipple className="tm-btn tm-btn-secondary" disabled={scanning} onClick={handleScan}>
              {scanning ? t('home.devices.importScanning') : t('home.devices.importScan')}
            </Button>
            <Typography className="tm-home-import-error">{error}</Typography>
          </Box>
          {summary ? <Typography className="tm-job-muted">{summary}</Typography> : null}
          {items.length > 0 ? (
            <ImportList items={items} query={query} selectedKeys={selectedKeys} onToggle={toggleOne} onQueryChange={setQuery} onChangeSelectedKeys={setSelectedKeys} />
          ) : scanned || !pasteable ? (
            <Box className="tm-job-table-wrap tm-home-import-placeholder">
              <Typography className="tm-job-muted">{scanned ? t('home.devices.importEmpty') : t('home.devices.importIdle')}</Typography>
            </Box>
          ) : null}
        </Box>
        <Box className="tm-edit-dialog-footer">
          <Button disableRipple className="tm-btn tm-btn-secondary" onClick={onClose}>
            {t('common.cancel')}
          </Button>
          <Box className="tm-edit-dialog-footer-spacer" />
          <Button disableRipple className="tm-btn tm-btn-primary" disabled={selectedVisibleKeys.length === 0 || saving} onClick={handleConfirm}>
            {saving ? t('home.devices.importSaving') : tf('home.devices.importConfirm', { n: selectedVisibleKeys.length })}
          </Button>
        </Box>
      </Box>
    </Dialog>
  );
}
