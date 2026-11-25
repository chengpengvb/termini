import type { RefObject } from 'react';
import { useEffect, useState } from 'react';
import Close from '@mui/icons-material/Close';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { isNotNil } from '@/common/util/StringUtil.ts';
import type { FileEntry } from '@/common/model/dto/FileEntry.ts';
import { useSftpService } from '@/common/context/useAppHooks.tsx';
import type { PermissionRow } from '@/page/common/dto/sftp/BodyMenu.ts';
import type { SshIdentityMaps } from '@/common/model/dto/SshIdentityMaps.ts';
import { Box, Button, Dialog, IconButton, Switch, Typography } from '@mui/material';
import { OverlayPaper, resolveScopedDialogProps } from '@/page/common/util/scopedDialogUtil.ts';

interface ChmodDialogProps {
  path: string;
  sessionId: string;
  entries: FileEntry[];
  onCancel: () => void;
  onConfirm: (mode: number) => void;
  dialogContainerRef?: RefObject<HTMLElement | null>;
}

export function BodyMenuChmod({ path, sessionId, entries, dialogContainerRef, onCancel, onConfirm }: ChmodDialogProps) {
  const { t } = useI18n();
  const firstEntry = entries[0];
  const sftpService = useSftpService();
  const scopedDialogProps = resolveScopedDialogProps(dialogContainerRef ? dialogContainerRef.current : null);

  const PERMISSION_ROWS: PermissionRow[] = [
    { label: 'Owner', read: 0o400, write: 0o200, execute: 0o100 },
    { label: 'Group', read: 0o040, write: 0o020, execute: 0o010 },
    { label: 'Others', read: 0o004, write: 0o002, execute: 0o001 },
  ];

  const resolveInitialMode = (): number => {
    if (!firstEntry) {
      return 0o644;
    }
    if (isNotNil(firstEntry.permissions)) {
      return firstEntry.permissions & 0o7777;
    }
    return 0o644;
  };

  const [mode, setMode] = useState<number>(resolveInitialMode);
  const [identityMap, setIdentityMap] = useState<SshIdentityMaps | null>(null);

  const toggleBit = (bit: number): void => {
    setMode(prev => (prev & bit ? prev & ~bit : prev | bit));
  };

  const hasBit = (bit: number): boolean => (mode & bit) !== 0;

  const resolveMappedName = (map: Record<string, string> | undefined, id: number): string | null => {
    if (!map) {
      return null;
    }
    const name = map[String(id)];
    if (!name) {
      return null;
    }
    return name;
  };

  const resolveDisplayUser = (): string | null => {
    if (!firstEntry) {
      return null;
    }
    if (firstEntry.user) {
      return firstEntry.user;
    }
    if (isNotNil(firstEntry.uid)) {
      const mappedName = resolveMappedName(identityMap?.uidToName, firstEntry.uid);
      if (mappedName) {
        return mappedName;
      }
      return String(firstEntry.uid);
    }
    return null;
  };

  const resolveDisplayGroup = (): string | null => {
    if (!firstEntry) {
      return null;
    }
    if (firstEntry.group) {
      return firstEntry.group;
    }
    if (isNotNil(firstEntry.gid)) {
      const mappedName = resolveMappedName(identityMap?.gidToName, firstEntry.gid);
      if (mappedName) {
        return mappedName;
      }
      return String(firstEntry.gid);
    }
    return null;
  };

  useEffect(() => {
    let cancelled = false;
    const deviceId = sftpService.getDeviceId(sessionId);
    if (!deviceId) {
      return;
    }
    sftpService.getIdentityMap(deviceId).then(map => {
      if (cancelled) {
        return;
      }
      setIdentityMap(map);
    });
    return () => {
      cancelled = true;
    };
  }, [sessionId]);

  const isSingleEntry = entries.length === 1;
  const displayUser = resolveDisplayUser();
  const displayGroup = resolveDisplayGroup();
  const octalDisplay = mode.toString(8).padStart(4, '0');
  const displayPath = isSingleEntry && firstEntry ? firstEntry.path : path;
  const showOwnership = isSingleEntry && (displayUser !== null || displayGroup !== null);

  return (
    <Dialog
      open
      onClose={onCancel}
      container={scopedDialogProps.container}
      disablePortal={scopedDialogProps.disablePortal}
      disableAutoFocus={scopedDialogProps.disableAutoFocus}
      disableScrollLock={scopedDialogProps.disableScrollLock}
      disableRestoreFocus={scopedDialogProps.disableRestoreFocus}
      disableEnforceFocus={scopedDialogProps.disableEnforceFocus}
      className={scopedDialogProps.className}
      sx={scopedDialogProps.sx}
      slots={{ paper: OverlayPaper }}
      slotProps={{ backdrop: scopedDialogProps.backdropSlotProps, paper: { className: 'tm-dialog-paper', sx: { width: 'min(100vw - 32px, 420px)', maxWidth: 'calc(100vw - 32px)' } } }}
    >
      <Box sx={{ px: 2.5, pt: 2, pb: 0.75, display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <Typography sx={{ fontSize: 15, fontWeight: 700, color: 'var(--tm-text-primary)' }}>{t('protocol.sftp.chmod')}</Typography>
        <IconButton size="small" onClick={onCancel} sx={{ color: 'var(--tm-text-tertiary)', '&:hover': { bgcolor: 'var(--tm-bg-hover)' } }}>
          <Close sx={{ fontSize: 16 }} />
        </IconButton>
      </Box>
      <Box sx={{ px: 2.5, pb: 1.5 }}>
        <Typography sx={{ fontSize: 12, color: 'var(--tm-text-secondary)', wordBreak: 'break-all' }}>{displayPath}</Typography>
      </Box>
      <Box sx={{ borderTop: '1px solid var(--tm-border-subtle)', overflowX: 'auto' }}>
        <Box sx={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1fr) 80px 80px 88px', minWidth: 280, px: 2.5, py: 1 }}>
          <Typography sx={{ fontSize: 12, fontWeight: 600, color: 'var(--tm-text-secondary)' }}>File Access</Typography>
          {(['Read', 'Write', 'Execute'] as const).map(h => (
            <Typography key={h} sx={{ fontSize: 12, fontWeight: 600, color: 'var(--tm-text-secondary)', textAlign: 'center' }}>
              {h}
            </Typography>
          ))}
        </Box>
        {PERMISSION_ROWS.map(row => (
          <Box key={row.label} sx={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1fr) 80px 80px 88px', minWidth: 280, px: 2.5, py: 0.25, alignItems: 'center', borderTop: '1px solid var(--tm-border-subtle)' }}>
            <Typography sx={{ fontSize: 13, color: 'var(--tm-text-primary)' }}>{row.label}</Typography>
            <Box sx={{ display: 'flex', justifyContent: 'center' }}>
              <Switch size="small" checked={hasBit(row.read)} onChange={() => toggleBit(row.read)} />
            </Box>
            <Box sx={{ display: 'flex', justifyContent: 'center' }}>
              <Switch size="small" checked={hasBit(row.write)} onChange={() => toggleBit(row.write)} />
            </Box>
            <Box sx={{ display: 'flex', justifyContent: 'center' }}>
              <Switch size="small" checked={hasBit(row.execute)} onChange={() => toggleBit(row.execute)} />
            </Box>
          </Box>
        ))}
      </Box>
      {showOwnership ? (
        <Box>
          <Box sx={{ px: 2.5, pt: 1.5, pb: 0.5 }}>
            <Typography sx={{ fontSize: 12, fontWeight: 600, color: 'var(--tm-text-secondary)' }}>Ownership</Typography>
          </Box>
          {displayUser !== null ? (
            <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', px: 2.5, py: 1, borderTop: '1px solid var(--tm-border-subtle)' }}>
              <Typography sx={{ fontSize: 13, color: 'var(--tm-text-secondary)' }}>User</Typography>
              <Typography sx={{ fontSize: 13, color: 'var(--tm-text-primary)' }}>{displayUser}</Typography>
            </Box>
          ) : null}
          {displayGroup !== null ? (
            <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', px: 2.5, py: 1, borderTop: '1px solid var(--tm-border-subtle)' }}>
              <Typography sx={{ fontSize: 13, color: 'var(--tm-text-secondary)' }}>Group</Typography>
              <Typography sx={{ fontSize: 13, color: 'var(--tm-text-primary)' }}>{displayGroup}</Typography>
            </Box>
          ) : null}
        </Box>
      ) : null}
      <Box sx={{ px: 2.5, py: 1.25, display: 'flex', justifyContent: 'space-between', alignItems: 'center', borderTop: '1px solid var(--tm-border-subtle)' }}>
        <Typography sx={{ fontSize: 12, color: 'var(--tm-text-tertiary)', fontFamily: 'monospace' }}>{octalDisplay}</Typography>
        <Box sx={{ display: 'flex', gap: 1 }}>
          <Button onClick={onCancel} sx={{ fontSize: 13, color: 'var(--tm-text-secondary)' }}>
            {t('common.cancel')}
          </Button>
          <Button variant="contained" onClick={() => onConfirm(mode)} sx={{ fontSize: 13, bgcolor: 'var(--tm-bg-selected)', color: 'var(--tm-text-primary)', '&:hover': { bgcolor: 'var(--tm-bg-selected)', filter: 'brightness(1.08)' } }}>
            {t('common.save')}
          </Button>
        </Box>
      </Box>
    </Dialog>
  );
}
