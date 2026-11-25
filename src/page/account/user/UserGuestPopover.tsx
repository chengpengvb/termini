import { Person } from '@mui/icons-material';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { Box, Popover, Typography } from '@mui/material';
import { useUserService } from '@/common/context/useAppHooks.tsx';
import { usePlatformMobile } from '@/common/context/usePlatformMobile';
import { ActionSheet } from '@/page/common/components/dialog/ActionSheet.tsx';

interface UserGuestPopoverProps {
  open: boolean;
  onClose: () => void;
  anchorEl: HTMLElement | null;
}

export function UserGuestPopover(props: UserGuestPopoverProps) {
  const { t } = useI18n();
  const userService = useUserService();
  const { open, anchorEl, onClose } = props;
  const mobileViewport = usePlatformMobile();

  const handleLogin = (): void => {
    onClose();
    userService.setLoginOpen(true);
  };

  const renderBody = () => {
    return (
      <Box className="tm-user-popover">
        <Box className="tm-user-popover-head">
          <Box className="tm-user-popover-avatar" sx={{ bgcolor: 'var(--tm-bg-hover)', color: 'var(--tm-text-secondary)' }}>
            <Person sx={{ fontSize: 20 }} />
          </Box>
          <Box className="tm-user-popover-identity">
            <Typography className="tm-user-popover-email">{t('account.user.notSignedIn')}</Typography>
          </Box>
          <Box className="tm-user-popover-exit" onClick={handleLogin}>
            {t('account.login.signIn')}
          </Box>
        </Box>
        <Box className="tm-user-popover-row">
          <Typography className="tm-user-popover-label">{t('account.user.cloudSync')}</Typography>
          <Box className="tm-user-popover-sync-actions">
            <Box className="tm-user-popover-sync-action is-disabled">{t('account.user.upload')}</Box>
            <Box className="tm-user-popover-sync-action is-disabled">{t('account.user.download')}</Box>
          </Box>
        </Box>
      </Box>
    );
  };

  if (mobileViewport) {
    return (
      <ActionSheet open={open} onClose={onClose}>
        {renderBody()}
      </ActionSheet>
    );
  }

  if (!open) {
    return null;
  }

  return (
    <Popover
      open
      disableScrollLock
      disableRestoreFocus
      anchorEl={anchorEl}
      onClose={onClose}
      anchorOrigin={{ vertical: 'bottom', horizontal: 'right' }}
      transformOrigin={{ vertical: 'top', horizontal: 'right' }}
      slotProps={{ paper: { className: 'tm-user-popover-paper', sx: { mt: 0.5 } } }}
    >
      <Box data-tm-overlay="true">{renderBody()}</Box>
    </Popover>
  );
}
