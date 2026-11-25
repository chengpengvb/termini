import { Box } from '@mui/material';
import type { RefObject } from 'react';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { useImperativeHandle, useState } from 'react';
import { useSftpService } from '@/common/context/useAppHooks.tsx';
import { FILE_KIND, FileEntry } from '@/common/model/dto/FileEntry.ts';
import { usePlatformMobile } from '@/common/context/usePlatformMobile';
import type { SftpHandle } from '@/page/common/dto/sftp/SftpHandle.ts';
import { EditField } from '@/page/common/components/input/EditField.tsx';
import { ConfirmDialog } from '@/common/components/dialog/ConfirmDialog.tsx';
import { BodyMenuChmod } from '@/page/protocol/sftp/transfer/part/body/BodyMenuChmod.tsx';
import { doDelete, doTransfer, isMenuNameValid, resolveMenuNameError, submitChmod, submitCreate, submitRename } from '@/page/common/util/sftpMenuUtil.ts';
import type { BodyMenuDialogsHandle, ChmodDialogState, CreateDialogState, DeleteDialogState, RenameDialogState, TransferDialogState } from '@/page/common/dto/sftp/BodyMenu.ts';

interface BodyMenuDialogsProps {
  sessionId: string;
  entries: FileEntry[];
  sideRef?: RefObject<SftpHandle | null>;
  dialogsRef: RefObject<BodyMenuDialogsHandle | null>;
  dialogContainerRef?: RefObject<HTMLElement | null>;
}

export function BodyMenuDialogs(props: BodyMenuDialogsProps) {
  const { t, tf } = useI18n();
  const sftpService = useSftpService();
  const mobileViewport = usePlatformMobile();
  const [chmodDialog, setChmodDialog] = useState<ChmodDialogState | null>(null);
  const [renameDialog, setRenameDialog] = useState<RenameDialogState | null>(null);
  const [createDialog, setCreateDialog] = useState<CreateDialogState | null>(null);
  const [deleteDialog, setDeleteDialog] = useState<DeleteDialogState | null>(null);
  const [transferDialog, setTransferDialog] = useState<TransferDialogState | null>(null);
  const { sessionId, entries, sideRef, dialogsRef, dialogContainerRef } = props;

  function requestTransfer(names: string[]): void {
    if (!names || names.length === 0) {
      return;
    }
    if (!mobileViewport) {
      doTransfer(sftpService, sessionId, sideRef, names);
      return;
    }
    const targetPath = sideRef?.current?.getCurrentPath();
    if (!targetPath) {
      console.warn('[sftp] copy to peer skipped: peer pane has no loaded directory');
      sftpService.notifyLocalError(sessionId, t('protocol.sftp.notReady'));
      return;
    }
    setTransferDialog({ names, targetPath });
  }

  function handleCancelTransfer(): void {
    setTransferDialog(null);
  }

  function handleConfirmTransfer(): void {
    if (!transferDialog) {
      return;
    }
    doTransfer(sftpService, sessionId, sideRef, transferDialog.names);
    setTransferDialog(null);
  }

  function openRename(selectedEntries: FileEntry[]): void {
    if (selectedEntries.length !== 1) {
      return;
    }
    const entry = selectedEntries[0];
    setRenameDialog({ entry, name: entry.name });
  }

  function handleCancelRename(): void {
    setRenameDialog(null);
  }

  function handleConfirmRename(): void {
    if (!renameDialog) {
      return;
    }
    if (!isMenuNameValid(entries, renameDialog.name, renameDialog.entry.name)) {
      return;
    }
    submitRename(sftpService, sessionId, renameDialog.entry, renameDialog.name.trim());
    setRenameDialog(null);
  }

  function openCreate(kind: typeof FILE_KIND.FILE | typeof FILE_KIND.DIRECTORY): void {
    setCreateDialog({ kind, name: '' });
  }

  function handleCancelCreate(): void {
    setCreateDialog(null);
  }

  function handleConfirmCreate(): void {
    if (!createDialog) {
      return;
    }
    if (!isMenuNameValid(entries, createDialog.name, null)) {
      return;
    }
    submitCreate(sftpService, sessionId, createDialog.kind, createDialog.name.trim());
    setCreateDialog(null);
  }

  function openDelete(selectedEntries: FileEntry[]): void {
    if (!selectedEntries || selectedEntries.length === 0) {
      return;
    }
    setDeleteDialog({ entries: selectedEntries });
  }

  function handleCancelDelete(): void {
    setDeleteDialog(null);
  }

  function handleConfirmDelete(): void {
    if (!deleteDialog) {
      return;
    }
    doDelete(sftpService, sessionId, deleteDialog.entries);
    setDeleteDialog(null);
  }

  function resolveDeleteDialogMessage(): string {
    if (!deleteDialog) {
      return '';
    }
    if (deleteDialog.entries.length === 1) {
      return tf('protocol.sftp.deleteOne', { name: deleteDialog.entries[0].name });
    }
    return tf('protocol.sftp.deleteMany', { n: deleteDialog.entries.length });
  }

  function openChmod(selectedEntries: FileEntry[]): void {
    const path = sftpService.getCurrentPath(sessionId);
    if (!path) {
      return;
    }
    setChmodDialog({ path, entries: selectedEntries });
  }

  function handleCancelChmod(): void {
    setChmodDialog(null);
  }

  function handleConfirmChmod(mode: number): void {
    if (!chmodDialog) {
      return;
    }
    submitChmod(sftpService, sessionId, chmodDialog.path, chmodDialog.entries, mode);
    setChmodDialog(null);
  }

  function resolveCreateDialogTitle(): string {
    if (!createDialog) {
      return '';
    }
    if (createDialog.kind === FILE_KIND.DIRECTORY) {
      return t('protocol.sftp.newFolder');
    }
    return t('protocol.sftp.newFile');
  }

  function resolveCreateDialogMessage(): string {
    if (!createDialog) {
      return '';
    }
    if (createDialog.kind === FILE_KIND.DIRECTORY) {
      return t('protocol.sftp.newFolderName');
    }
    return t('protocol.sftp.newFileName');
  }

  useImperativeHandle(dialogsRef, () => ({
    openRename,
    openCreate,
    openDelete,
    openChmod,
    requestTransfer,
  }));

  return (
    <>
      <ConfirmDialog
        open={renameDialog !== null}
        title={t('protocol.sftp.rename')}
        message={t('protocol.sftp.renameMessage')}
        confirmLabel={t('common.confirm')}
        error={renameDialog ? resolveMenuNameError(entries, renameDialog.name, renameDialog.entry.name, t('common.nameCannotContainSlash'), t('common.nameExists')) : null}
        confirmDisabled={renameDialog ? !isMenuNameValid(entries, renameDialog.name, renameDialog.entry.name) : true}
        dialogContainerRef={dialogContainerRef}
        onCancel={handleCancelRename}
        onConfirm={handleConfirmRename}
      >
        {renameDialog ? (
          <Box sx={{ mt: 1.5 }}>
            <EditField autoFocus value={renameDialog.name} onChange={name => setRenameDialog({ entry: renameDialog.entry, name })} />
          </Box>
        ) : null}
      </ConfirmDialog>
      <ConfirmDialog
        open={createDialog !== null}
        title={resolveCreateDialogTitle()}
        message={resolveCreateDialogMessage()}
        confirmLabel={t('common.confirm')}
        error={createDialog ? resolveMenuNameError(entries, createDialog.name, null, t('common.nameCannotContainSlash'), t('common.nameExists')) : null}
        confirmDisabled={createDialog ? !isMenuNameValid(entries, createDialog.name, null) : true}
        dialogContainerRef={dialogContainerRef}
        onCancel={handleCancelCreate}
        onConfirm={handleConfirmCreate}
      >
        {createDialog ? (
          <Box sx={{ mt: 1.5 }}>
            <EditField autoFocus value={createDialog.name} onChange={name => setCreateDialog({ kind: createDialog.kind, name })} />
          </Box>
        ) : null}
      </ConfirmDialog>
      <ConfirmDialog open={deleteDialog !== null} danger title={t('common.delete')} message={resolveDeleteDialogMessage()} confirmLabel={t('common.delete')} dialogContainerRef={dialogContainerRef} onCancel={handleCancelDelete} onConfirm={handleConfirmDelete} />
      <ConfirmDialog open={transferDialog !== null} title={t('protocol.sftp.copyToPeer')} message={t('protocol.sftp.targetFolder')} confirmLabel={t('protocol.sftp.copyToPeer')} dialogContainerRef={dialogContainerRef} onCancel={handleCancelTransfer} onConfirm={handleConfirmTransfer}>
        {transferDialog ? <Box sx={{ mt: 1.5, fontSize: 13, color: 'var(--tm-text-primary)', wordBreak: 'break-all', bgcolor: 'var(--tm-bg-elevated)', p: 1, borderRadius: 'var(--tm-dialog-radius)' }}>{transferDialog.targetPath}</Box> : null}
      </ConfirmDialog>
      {chmodDialog ? <BodyMenuChmod path={chmodDialog.path} sessionId={sessionId} entries={chmodDialog.entries} dialogContainerRef={dialogContainerRef} onCancel={handleCancelChmod} onConfirm={handleConfirmChmod} /> : null}
    </>
  );
}
