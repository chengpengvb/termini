import { Sync } from '@mui/icons-material';
import { useEffect, useState } from 'react';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { formatDate } from '@/common/util/DateUtil.ts';
import { MergeDialog } from '@/page/account/merge/MergeDialog.tsx';
import { usePlatformMobile } from '@/common/context/usePlatformMobile';
import { AlertDialog } from '@/common/components/dialog/AlertDialog.tsx';
import { Box, LinearProgress, Popover, Typography } from '@mui/material';
import { EditField } from '@/page/common/components/input/EditField.tsx';
import type { UserAccount } from '@/common/model/resp/RespUserAccount.ts';
import { ConfirmDialog } from '@/common/components/dialog/ConfirmDialog.tsx';
import { ActionSheet } from '@/page/common/components/dialog/ActionSheet.tsx';
import { BizError, resolveErrorText } from '@/common/model/resp/ResultModel.ts';
import type { RespPlatformInfo } from '@/common/model/resp/RespPlatformInfo.ts';
import { AccountDeleteDialog } from '@/page/account/user/AccountDeleteDialog.tsx';
import type { UserSubscription } from '@/common/model/resp/RespUserSubscription.ts';
import type { CloudSyncChoice, CloudSyncConflict } from '@/common/model/dto/CloudSync.ts';
import { useAccountService, useSystemUtilService } from '@/common/context/useAppHooks.tsx';
import { resolveKeyPairAvatarStyle, resolveKeyPairAvatarLetter } from '@/page/common/util/keyPairUtil.ts';
import { CLOUD_SYNC_OP, CLOUD_SYNC_PASSWORD_MAX_LEN, CLOUD_SYNC_PASSWORD_MIN_LEN } from '@/common/model/dto/CloudSync.ts';
import { formatCredits, isProActive, resolveSubscriptionStatus, SUBSCRIPTION_STATUS, SUBSCRIPTION_STORE } from '@/page/common/util/subscriptionStatus.ts';

interface UserInfoPopoverProps {
  open: boolean;
  onClose: () => void;
  account: UserAccount;
  anchorEl: HTMLElement | null;
}

export function UserInfoPopover(props: UserInfoPopoverProps) {
  const { t, tf } = useI18n();
  const email = props.account.email;
  const { open, anchorEl, onClose } = props;
  const mobileViewport = usePlatformMobile();
  const accountService = useAccountService();
  const [password, setPassword] = useState('');
  const subscription = props.account.subscription;
  const systemUtilService = useSystemUtilService();
  const [restoring, setRestoring] = useState(false);
  const [deleteOpen, setDeleteOpen] = useState(false);
  const [subscribing, setSubscribing] = useState(false);
  const status = resolveSubscriptionStatus(subscription);
  const [error, setError] = useState<string | null>(null);
  const [syncStatusTime, setSyncStatusTime] = useState(0);
  const [syncScene, setSyncScene] = useState<string | null>(null);
  const [alertMessage, setAlertMessage] = useState<string | null>(null);
  const [passwordOpen, setPasswordOpen] = useState<number | null>(null);
  const [platform, setPlatform] = useState<RespPlatformInfo | null>(null);
  const [mergeConflicts, setMergeConflicts] = useState<CloudSyncConflict[]>([]);
  const [syncing, setSyncing] = useState<number | null>(accountService.getCloudSyncing());
  const [pendingSync, setPendingSync] = useState<{ op: number; password: string } | null>(null);
  const storeReady = platform?.has_store === true;
  const syncAllowed = isProActive(subscription);

  const resolveStatusLabel = (): string => {
    if (status === SUBSCRIPTION_STATUS.ACTIVE) {
      return t('account.user.proPlan');
    }
    return t('account.user.freePlan');
  };

  const resolveExpireHint = (item: UserSubscription): string => {
    if (status === SUBSCRIPTION_STATUS.ACTIVE) {
      return tf('account.user.expireAt', { date: formatDate(item.expireTime) });
    }
    return tf('account.user.quotaReset', { date: formatDate(item.quotaResetTime) });
  };

  const resolveManagedHint = (item: UserSubscription): string | null => {
    if (item.store === SUBSCRIPTION_STORE.APPLE) {
      return t('account.user.managedByApple');
    }
    if (item.store === SUBSCRIPTION_STORE.MICROSOFT) {
      return t('account.user.managedByMicrosoft');
    }
    if (item.store === SUBSCRIPTION_STORE.GOOGLE) {
      return t('account.user.managedByGoogle');
    }
    return null;
  };

  const resolveStoreErrorText = (e: unknown): string => {
    const raw = e instanceof BizError ? e.bizMessage : e instanceof Error ? e.message : null;
    if (raw != null && (raw.includes('not supported on this platform') || raw.includes('store not ready'))) {
      return t('account.subscription.storeOnly');
    }
    return resolveErrorText(e);
  };

  const handleSubscribe = (): void => {
    if (subscribing || restoring || !storeReady) {
      return;
    }
    setSubscribing(true);
    accountService
      .subscribePro()
      .catch(e => {
        setAlertMessage(resolveStoreErrorText(e));
      })
      .finally(() => {
        setSubscribing(false);
      });
  };

  const handleRestore = (): void => {
    if (subscribing || restoring) {
      return;
    }
    setRestoring(true);
    accountService
      .restorePro()
      .catch(e => {
        setAlertMessage(resolveStoreErrorText(e));
      })
      .finally(() => {
        setRestoring(false);
      });
  };

  const resolveUsedPercent = (item: UserSubscription): number => {
    if (item.monthlyQuota === 0) {
      return 100;
    }
    return (item.usedQuota / item.monthlyQuota) * 100;
  };

  const resolveSubscribeClass = (): string => {
    if (subscribing || restoring || !storeReady) {
      return 'tm-user-popover-subscribe-btn is-busy';
    }
    return 'tm-user-popover-subscribe-btn';
  };

  const resolveRestoreClass = (): string => {
    if (subscribing || restoring || !storeReady) {
      return 'tm-user-popover-restore is-busy';
    }
    return 'tm-user-popover-restore';
  };

  const resolveSyncActionClass = (): string => {
    if (!syncAllowed) {
      return 'tm-user-popover-sync-action is-disabled';
    }
    if (syncing == null) {
      return 'tm-user-popover-sync-action';
    }
    return 'tm-user-popover-sync-action is-busy';
  };

  const resolvePasswordConfirmLabel = (op: number | null): string => {
    if (op === CLOUD_SYNC_OP.DOWNLOAD) {
      return t('account.user.download');
    }
    return t('account.user.upload');
  };

  const handlePopoverClose = (): void => {
    if (passwordOpen != null || syncing != null || deleteOpen) {
      return;
    }
    onClose();
  };

  const handleDeleteOpen = (): void => {
    setDeleteOpen(true);
  };

  const handleDeleteClose = (): void => {
    setDeleteOpen(false);
    onClose();
  };

  const handleLogout = (): void => {
    void accountService.logout();
    onClose();
  };

  const handleOpenPortal = (): void => {
    accountService.openPortal().catch(e => {
      setAlertMessage(resolveErrorText(e));
    });
  };

  const openSyncPassword = async (op: number): Promise<void> => {
    try {
      const status = await accountService.cloudSyncStatus();
      if (op === CLOUD_SYNC_OP.DOWNLOAD && !status.hasBackup) {
        setAlertMessage(t('account.user.syncNoBackup'));
        return;
      }
      if (op === CLOUD_SYNC_OP.UPLOAD) {
        setSyncScene(status.hasBackup ? 'overwriteUpload' : 'firstUpload');
      } else {
        setSyncScene('download');
      }
      setSyncStatusTime(status.updateTime);
      setError(null);
      setPassword('');
      setPasswordOpen(op);
    } catch (e) {
      setAlertMessage(resolveErrorText(e));
    }
  };

  const resolveSyncMessage = (): string => {
    if (syncScene === 'firstUpload') {
      return t('account.user.syncFirstUploadMessage');
    }
    if (syncScene === 'overwriteUpload') {
      return tf('account.user.syncOverwriteUploadMessage', { date: formatDate(syncStatusTime) });
    }
    if (syncScene === 'download') {
      return tf('account.user.syncDownloadMessage', { date: formatDate(syncStatusTime) });
    }
    return t('account.user.syncPasswordMessage');
  };

  const handleUploadClick = (): void => {
    if (syncing != null || !syncAllowed) {
      return;
    }
    void openSyncPassword(CLOUD_SYNC_OP.UPLOAD);
  };

  const handleDownloadClick = (): void => {
    if (syncing != null || !syncAllowed) {
      return;
    }
    void openSyncPassword(CLOUD_SYNC_OP.DOWNLOAD);
  };

  const handlePasswordChange = (value: string): void => {
    setError(null);
    setPassword(value);
  };

  const handlePasswordCancel = (): void => {
    setPasswordOpen(null);
    setPassword('');
    setError(null);
  };

  const runSync = async (op: number, nextPassword: string): Promise<void> => {
    try {
      const conflicts = await accountService.prepareCloudSync(op, nextPassword);
      if (conflicts.length === 0) {
        await accountService.applyCloudSync(op, nextPassword, []);
        return;
      }
      setPendingSync({ op, password: nextPassword });
      setMergeConflicts(conflicts);
    } catch (e) {
      setAlertMessage(resolveErrorText(e));
    }
  };

  const handleMergeResolved = (choices: CloudSyncChoice[]): void => {
    if (pendingSync == null) {
      throw new Error('cloud sync pending state is required');
    }
    const pending = pendingSync;
    setMergeConflicts([]);
    setPendingSync(null);
    accountService.applyCloudSync(pending.op, pending.password, choices).catch(e => {
      setAlertMessage(resolveErrorText(e));
    });
  };

  const handleMergeCancel = (): void => {
    setMergeConflicts([]);
    setPendingSync(null);
    accountService.cancelCloudSync();
  };

  const handlePasswordConfirm = (): void => {
    if (syncing != null) {
      return;
    }
    if (password.length < CLOUD_SYNC_PASSWORD_MIN_LEN) {
      setError(t('account.user.passwordMin'));
      return;
    }
    if (password.length > CLOUD_SYNC_PASSWORD_MAX_LEN) {
      setError(t('account.user.passwordMax'));
      return;
    }
    const op = passwordOpen;
    const nextPassword = password;
    if (op == null) {
      throw new Error('cloud sync op is required');
    }
    setPasswordOpen(null);
    setPassword('');
    setError(null);
    void runSync(op, nextPassword);
  };

  const handleAlertClose = (): void => {
    setAlertMessage(null);
  };

  useEffect(() => {
    return accountService.addCloudSyncingListener(setSyncing);
  }, [accountService]);

  useEffect(() => {
    void systemUtilService.getPlatformInfo().then(info => {
      setPlatform(info);
    });
  }, [systemUtilService]);

  if (email == null || email.length === 0) {
    throw new Error('account.email is required');
  }

  const balanceText = subscription == null || subscription.balanceQuota === 0 ? null : formatCredits(subscription.balanceQuota, props.account.creditScale);

  const renderBody = () => {
    return (
        <Box className="tm-user-popover">
          <Box className="tm-user-popover-head">
            <Box className="tm-user-popover-avatar tm-user-popover-clickable" style={resolveKeyPairAvatarStyle(email)} sx={{ bgcolor: 'var(--tm-device-os-color)' }} onClick={handleOpenPortal} title={t('account.user.openPortal')}>
              {resolveKeyPairAvatarLetter(email)}
            </Box>
            <Box className="tm-user-popover-identity">
              <Typography className="tm-user-popover-email tm-user-popover-clickable" onClick={handleOpenPortal} title={t('account.user.openPortal')}>
                {email}
              </Typography>
              {subscription != null ? <Typography className="tm-user-popover-plan-tag">{resolveStatusLabel()}</Typography> : null}
            </Box>
            <Box className="tm-user-popover-exit" onClick={handleLogout}>
              {t('account.user.logout')}
            </Box>
          </Box>
          {subscription != null ? (
            <>
              <Box className="tm-user-popover-plan-card">
                <Typography className="tm-user-popover-kicker">{t('account.user.currentPlan')}</Typography>
                <Typography className="tm-user-popover-plan-name">{resolveStatusLabel()}</Typography>
                <Typography className="tm-user-popover-hint">{resolveExpireHint(subscription)}</Typography>
                {status === SUBSCRIPTION_STATUS.ACTIVE ? (
                  resolveManagedHint(subscription) != null ? (
                    <Typography className="tm-user-popover-hint">{resolveManagedHint(subscription)}</Typography>
                  ) : null
                ) : storeReady ? (
                  <>
                    <Box className={resolveSubscribeClass()} onClick={handleSubscribe}>
                      {subscribing ? t('account.subscription.subscribing') : t('account.user.upgradePro')}
                    </Box>
                    <Typography className={resolveRestoreClass()} onClick={handleRestore}>
                      {restoring ? t('account.subscription.restoring') : t('account.subscription.restore')}
                    </Typography>
                  </>
                ) : null}
              </Box>
              <Box className="tm-user-popover-section">
                <Typography className="tm-user-popover-section-title">{t('account.user.includedQuota')}</Typography>
                <Box className="tm-user-popover-row">
                  <Typography className="tm-user-popover-label">{t('account.user.monthlyQuota')}</Typography>
                  <Typography className="tm-user-popover-value">{tf('account.user.usedPercent', { n: resolveUsedPercent(subscription).toFixed(0) })}</Typography>
                </Box>
                <LinearProgress className="tm-user-popover-bar" variant="determinate" value={resolveUsedPercent(subscription)} />
                <Typography className="tm-user-popover-hint">{t('account.user.monthlyQuotaHint')}</Typography>
                {balanceText === null || balanceText === '0' ? null : (
                  <>
                    <Box className="tm-user-popover-row">
                      <Typography className="tm-user-popover-label">{t('account.user.balance')}</Typography>
                      <Typography className="tm-user-popover-value">{balanceText}</Typography>
                    </Box>
                    <Typography className="tm-user-popover-hint">{t('account.user.balanceHint')}</Typography>
                  </>
                )}
              </Box>
              <Box className="tm-user-popover-row">
                <Typography className="tm-user-popover-label">{t('account.user.cloudSync')}</Typography>
                <Box className="tm-user-popover-sync-actions">
                  <Box className={resolveSyncActionClass()} onClick={handleUploadClick}>
                    {syncing === CLOUD_SYNC_OP.UPLOAD ? <Sync className="tm-user-popover-sync-spin" sx={{ fontSize: 14 }} /> : null}
                    {t('account.user.upload')}
                  </Box>
                  <Box className={resolveSyncActionClass()} onClick={handleDownloadClick}>
                    {syncing === CLOUD_SYNC_OP.DOWNLOAD ? <Sync className="tm-user-popover-sync-spin" sx={{ fontSize: 14 }} /> : null}
                    {t('account.user.download')}
                  </Box>
                </Box>
              </Box>
            </>
          ) : null}
          <Box className="tm-user-popover-danger" onClick={handleDeleteOpen}>
            {t('account.delete.entry')}
          </Box>
        </Box>
    );
  };

  return (
    <>
      {mobileViewport ? (
        <ActionSheet open={open} onClose={handlePopoverClose}>
          {renderBody()}
        </ActionSheet>
      ) : open ? (
        <Popover
          open
          disableScrollLock
          disableRestoreFocus
          anchorEl={anchorEl}
          onClose={handlePopoverClose}
          anchorOrigin={{ vertical: 'bottom', horizontal: 'right' }}
          transformOrigin={{ vertical: 'top', horizontal: 'right' }}
          slotProps={{ paper: { className: 'tm-user-popover-paper', sx: { mt: 0.5 } } }}
        >
          <Box data-tm-overlay="true">{renderBody()}</Box>
        </Popover>
      ) : null}
      <ConfirmDialog open={passwordOpen != null} title={t('account.user.syncTitle')} message={resolveSyncMessage()} error={error} confirmLabel={resolvePasswordConfirmLabel(passwordOpen)} onCancel={handlePasswordCancel} onConfirm={handlePasswordConfirm}>
        <Box sx={{ mt: 1.5 }}>
          <EditField autoFocus password value={password} onChange={handlePasswordChange} placeholder={t('account.user.syncPasswordPlaceholder')} />
        </Box>
      </ConfirmDialog>
      <AccountDeleteDialog open={deleteOpen} email={email} onClose={handleDeleteClose} />
      <MergeDialog open={mergeConflicts.length > 0} conflicts={mergeConflicts} onCancel={handleMergeCancel} onResolved={handleMergeResolved} />
      <AlertDialog duration={5000} open={alertMessage != null} handleClose={handleAlertClose} message={alertMessage} type="error" />
    </>
  );
}
