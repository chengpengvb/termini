import { useEffect, useState } from 'react';
import { useI18n } from '@/common/i18n/useI18n.ts';
import type { Group } from '@/common/model/Group.ts';
import { isEmpty, isNotEmpty } from '@/common/util/StringUtil';
import { Header } from '@/page/common/components/dialog/Header.tsx';
import { resolveErrorText } from '@/common/model/resp/ResultModel.ts';
import { useBackDismiss } from '@/page/common/hooks/useBackDismiss.ts';
import { AlertDialog } from '@/common/components/dialog/AlertDialog.tsx';
import { EditField } from '@/page/common/components/input/EditField.tsx';
import { Box, Button, ButtonBase, Dialog, Typography } from '@mui/material';
import { ConfirmDialog } from '@/common/components/dialog/ConfirmDialog.tsx';
import { resolveSheetDialogProps } from '@/page/common/util/scopedDialogUtil.ts';
import { Add, DeleteOutlined, DriveFileRenameOutline } from '@mui/icons-material';
import { useDeviceService, useGroupService } from '@/common/context/useAppHooks.tsx';
import { SelectField, type SelectOption } from '@/page/common/components/input/SelectField.tsx';
import { DEFAULT_GROUP_ID, NEW_GROUP_ACTION_ID, resolveGroupName } from '@/common/model/Group.ts';

function sortGroups(list: Group[]): Group[] {
  return [...list].sort((left, right) => {
    if (left.id === DEFAULT_GROUP_ID) {
      return -1;
    }
    if (right.id === DEFAULT_GROUP_ID) {
      return 1;
    }
    return left.name.localeCompare(right.name);
  });
}

export function GroupFields(props: { value: string | null | undefined; onChange: (groupId: string) => void }) {
  const { t, tf } = useI18n();
  const { value, onChange } = props;
  const groupService = useGroupService();
  const deviceService = useDeviceService();
  const [editId, setEditId] = useState<string>('');
  const [groups, setGroups] = useState<Group[]>([]);
  const [editName, setEditName] = useState<string>('');
  const [deleteId, setDeleteId] = useState<string>('');
  const [createName, setCreateName] = useState<string>('');
  const [options, setOptions] = useState<SelectOption[]>([]);
  const [manageOpen, setManageOpen] = useState<boolean>(false);
  const [alertInfo, setAlertInfo] = useState<{ showAlert: boolean; message: string | null }>({ showAlert: false, message: null });

  const resolveValue = (): string => {
    if (isNotEmpty(value)) {
      return value;
    }
    return DEFAULT_GROUP_ID;
  };

  const applyGroups = (list: Group[]): void => {
    const sorted = sortGroups(list);
    setGroups(sorted);
    setOptions([{ value: NEW_GROUP_ACTION_ID, label: t('home.devices.groupNew') }, ...sorted.map(group => ({ value: group.id, label: resolveGroupName(group) }))]);
  };

  const reloadGroups = async (): Promise<void> => {
    try {
      applyGroups(await groupService.selectAll());
    } catch (err) {
      setAlertInfo({ showAlert: true, message: tf('home.common.loadFailedExclaim', { error: resolveErrorText(err) }) });
    }
  };

  const handleSelectChange = (nextValue: string): void => {
    if (nextValue === NEW_GROUP_ACTION_ID) {
      setCreateName('');
      setEditId('');
      setEditName('');
      setDeleteId('');
      setManageOpen(true);
      return;
    }
    onChange(nextValue);
  };

  const handleManageClose = (): void => {
    setManageOpen(false);
    setCreateName('');
    setEditId('');
    setEditName('');
    setDeleteId('');
  };

  const handleCreateSave = (): void => {
    const name = createName.trim();
    if (isEmpty(name)) {
      return;
    }
    groupService
      .insert({ id: String(), name })
      .then(saved => {
        return reloadGroups().then(() => {
          setCreateName('');
          onChange(saved.id);
        });
      })
      .catch(err => {
        setAlertInfo({ showAlert: true, message: tf('home.common.saveFailed', { error: resolveErrorText(err) }) });
      });
  };

  const handleRenameStart = (group: Group): void => {
    if (group.id === DEFAULT_GROUP_ID) {
      return;
    }
    setEditId(group.id);
    setEditName(group.name);
  };

  const handleRenameCancel = (): void => {
    setEditId('');
    setEditName('');
  };

  const handleRenameSave = (): void => {
    const name = editName.trim();
    if (isEmpty(editId) || isEmpty(name)) {
      return;
    }
    groupService
      .update({ id: editId, name })
      .then(() => {
        setEditId('');
        setEditName('');
        return reloadGroups();
      })
      .catch(err => {
        setAlertInfo({ showAlert: true, message: tf('home.common.saveFailed', { error: resolveErrorText(err) }) });
      });
  };

  const handleDeleteAsk = (group: Group): void => {
    if (group.id === DEFAULT_GROUP_ID) {
      return;
    }
    setDeleteId(group.id);
  };

  const handleDeleteCancel = (): void => {
    setDeleteId('');
  };

  const handleDeleteConfirm = (): void => {
    if (isEmpty(deleteId) || deleteId === DEFAULT_GROUP_ID) {
      return;
    }
    const removedId = deleteId;
    groupService
      .deleteById(removedId)
      .then(() => {
        setDeleteId('');
        if (resolveValue() === removedId) {
          onChange(DEFAULT_GROUP_ID);
        }
        return Promise.all([reloadGroups(), deviceService.selectAllWithConfig()]);
      })
      .catch(err => {
        setAlertInfo({ showAlert: true, message: tf('home.common.saveFailed', { error: resolveErrorText(err) }) });
      });
  };

  const handleAlertClose = (): void => {
    setAlertInfo({ showAlert: false, message: null });
  };

  useBackDismiss(manageOpen, handleManageClose, 'group-manage');

  useEffect(() => {
    void reloadGroups();
  }, [groupService, t]);

  return (
    <>
      <SelectField label={t('home.devices.groupLabel')} value={resolveValue()} onChange={handleSelectChange} options={options} placeholder={t('home.devices.groupPlaceholder')} />
      <Dialog open={manageOpen} onClose={handleManageClose} {...resolveSheetDialogProps({ className: 'tm-dialog-paper tm-group-manage-paper' })}>
        <Box className="tm-group-manage">
          <Header title={t('home.devices.groupCreateTitle')} onClose={handleManageClose} />
          <Box className="tm-group-manage-add">
            <EditField label={t('home.common.name')} value={createName} onChange={setCreateName} placeholder={t('home.devices.groupNamePlaceholder')} autoFocus actionIcon={<Add sx={{ fontSize: 18 }} />} actionLabel={t('common.add')} onAction={handleCreateSave} />
          </Box>
          <Box className="tm-set-saved-list tm-group-manage-list">
            {groups.map(group => (
              <Box key={group.id} className={group.id === resolveValue() ? 'tm-set-saved-item is-current' : 'tm-set-saved-item'}>
                {editId === group.id ? (
                  <Box className="tm-group-manage-edit">
                    <EditField value={editName} onChange={setEditName} placeholder={t('home.devices.groupNamePlaceholder')} autoFocus />
                    <Button disableRipple onClick={handleRenameCancel} className="tm-btn tm-btn-secondary">
                      {t('common.cancel')}
                    </Button>
                    <Button disableRipple disabled={isEmpty(editName.trim())} onClick={handleRenameSave} className="tm-btn tm-btn-primary">
                      {t('common.save')}
                    </Button>
                  </Box>
                ) : (
                  <>
                    <Box className="tm-set-saved-item-main">
                      <Typography className="tm-set-saved-item-title">{resolveGroupName(group)}</Typography>
                    </Box>
                    {group.id === DEFAULT_GROUP_ID ? null : (
                      <Box className="tm-set-saved-item-actions">
                        <ButtonBase disableRipple aria-label={t('common.edit')} className="tm-set-saved-item-action" onClick={() => handleRenameStart(group)}>
                          <DriveFileRenameOutline sx={{ fontSize: 16 }} />
                        </ButtonBase>
                        <ButtonBase disableRipple aria-label={t('common.delete')} className="tm-set-saved-item-action is-danger" onClick={() => handleDeleteAsk(group)}>
                          <DeleteOutlined sx={{ fontSize: 16 }} />
                        </ButtonBase>
                      </Box>
                    )}
                  </>
                )}
              </Box>
            ))}
          </Box>
        </Box>
      </Dialog>
      <ConfirmDialog open={isNotEmpty(deleteId)} title={t('home.devices.groupDeleteTitle')} message={t('home.devices.groupDeleteMessage')} danger confirmLabel={t('common.delete')} onCancel={handleDeleteCancel} onConfirm={handleDeleteConfirm} />
      <AlertDialog duration={1500} open={alertInfo.showAlert} handleClose={handleAlertClose} message={alertInfo.message} type="error" />
    </>
  );
}
