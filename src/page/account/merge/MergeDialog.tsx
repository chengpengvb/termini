import { useEffect, useState } from 'react';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { Box, Button, Dialog, Typography } from '@mui/material';
import { CLOUD_SYNC_SIDE } from '@/common/model/dto/CloudSync.ts';
import { resolveMergeEntry } from '@/page/common/util/mergeRegistry.ts';
import { resolveSheetDialogProps } from '@/page/common/util/scopedDialogUtil.ts';
import type { CloudSyncChoice, CloudSyncConflict } from '@/common/model/dto/CloudSync.ts';

export function MergeDialog(props: { open: boolean; conflicts: CloudSyncConflict[]; onCancel: () => void; onResolved: (choices: CloudSyncChoice[]) => void }) {
  const { t, tf } = useI18n();
  const [index, setIndex] = useState(0);
  const conflict = props.conflicts[index];
  const { open, conflicts, onCancel, onResolved } = props;
  const [choices, setChoices] = useState<CloudSyncChoice[]>([]);
  const entry = conflict ? resolveMergeEntry(conflict.sync_type) : null;
  const EntityComponent = entry ? entry.component : null;

  const handleChoose = (side: number): void => {
    if (!conflict) {
      return;
    }
    const nextChoices = [...choices, { sync_type: conflict.sync_type, id: conflict.id, side }];
    if (index + 1 >= conflicts.length) {
      onResolved(nextChoices);
      return;
    }
    setChoices(nextChoices);
    setIndex(index + 1);
  };

  useEffect(() => {
    if (!open) {
      return;
    }
    setIndex(0);
    setChoices([]);
  }, [open]);

  return (
    <Dialog open={open} onClose={onCancel} {...resolveSheetDialogProps({ className: 'tm-dialog-paper', sx: { width: 520 } })}>
      <Box sx={{ px: 2.5, pt: 2.25, pb: 0.5, display: 'flex', alignItems: 'baseline', gap: 1 }}>
        <Typography sx={{ fontSize: 15, fontWeight: 700, color: 'var(--tm-text-primary)' }}>{t('account.merge.title')}</Typography>
        {conflict ? <Typography sx={{ fontSize: 12, color: 'var(--tm-text-secondary)' }}>{tf('account.merge.progress', { index: index + 1, total: conflicts.length, label: entry ? t(entry.labelKey) : '' })}</Typography> : null}
      </Box>
      <Box sx={{ px: 2.5, pt: 1, pb: 1.5, maxHeight: 420, overflowY: 'auto' }}>{conflict && EntityComponent ? <EntityComponent local={conflict.local} cloud={conflict.cloud} /> : null}</Box>
      <Box className="tm-edit-dialog-footer">
        <Button disableRipple onClick={onCancel} className="tm-btn tm-btn-secondary">
          {t('common.cancel')}
        </Button>
        <Box className="tm-edit-dialog-footer-spacer" />
        <Button disableRipple onClick={() => handleChoose(CLOUD_SYNC_SIDE.LOCAL)} className="tm-btn tm-btn-secondary">
          {t('account.merge.useLocal')}
        </Button>
        <Button disableRipple variant="contained" onClick={() => handleChoose(CLOUD_SYNC_SIDE.CLOUD)} className="tm-btn tm-btn-primary">
          {t('account.merge.acceptCloud')}
        </Button>
      </Box>
    </Dialog>
  );
}
