import { Box } from '@mui/material';
import type { MouseEvent } from 'react';
import { Person } from '@mui/icons-material';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { useEffect, useRef, useState } from 'react';
import { UserInfoPopover } from './UserInfoPopover.tsx';
import { UserGuestPopover } from './UserGuestPopover.tsx';
import type { UserAccount } from '@/common/model/resp/RespUserAccount.ts';
import { useAccountService, useUserService } from '@/common/context/useAppHooks.tsx';
import { resolveKeyPairAvatarStyle, resolveKeyPairAvatarLetter } from '@/page/common/util/keyPairUtil.ts';

export function UserAvatarBadge() {
  const { t } = useI18n();
  const userService = useUserService();
  const accountService = useAccountService();
  const anchorRef = useRef<HTMLSpanElement>(null);
  const [open, setOpen] = useState<boolean>(() => userService.isPopoverOpen());
  const [account, setAccount] = useState<UserAccount | null | undefined>(() => accountService.getAccount());
  const email = account === null || account === undefined || account.email === null || account.email === undefined || account.email.length === 0 ? null : account.email;
  const signedIn = email !== null;

  const handleClose = (): void => {
    userService.setPopoverOpen(false);
  };

  const handleToggle = (event: MouseEvent): void => {
    event.stopPropagation();
    userService.togglePopover();
  };

  useEffect(() => {
    return accountService.addAccountChangeListener(setAccount);
  }, [accountService]);

  useEffect(() => {
    return userService.addPopoverListener(setOpen);
  }, [userService]);

  useEffect(() => {
    if (!open || !signedIn) {
      return;
    }
    void accountService.refreshAccount();
  }, [open, signedIn, accountService]);

  useEffect(() => {
    if (!open) {
      return;
    }
    const closeIfOutside = (event: Event): void => {
      const target = event.target;
      if (!(target instanceof Node)) {
        return;
      }
      if (anchorRef.current?.contains(target)) {
        return;
      }
      if (target instanceof Element && target.closest('[data-tm-overlay="true"]')) {
        return;
      }
      handleClose();
    };
    document.addEventListener('pointerdown', closeIfOutside, true);
    return () => {
      document.removeEventListener('pointerdown', closeIfOutside, true);
    };
  }, [open]);

  if (email === null || account === null || account === undefined) {
    return (
      <>
        <Box component="span" ref={anchorRef} className="tm-user-signed-out" title={t('account.login.signInUser')} onClick={handleToggle}>
          <Person />
          <Box className="tm-user-signed-out-tip" />
        </Box>
        <UserGuestPopover open={open} anchorEl={anchorRef.current} onClose={handleClose} />
      </>
    );
  }

  return (
    <>
      <Box
        component="span"
        ref={anchorRef}
        onClick={handleToggle}
        style={resolveKeyPairAvatarStyle(email)}
        sx={{ width: '100%', height: '100%', padding: '9px', boxSizing: 'border-box', backgroundClip: 'content-box', borderRadius: '50%', display: 'flex', alignItems: 'center', justifyContent: 'center', fontSize: 12, fontWeight: 700, color: 'var(--tm-icon-on-brand)', bgcolor: 'var(--tm-device-os-color)' }}
      >
        {resolveKeyPairAvatarLetter(email)}
      </Box>
      <UserInfoPopover open={open} anchorEl={anchorRef.current} account={account} onClose={handleClose} />
    </>
  );
}
