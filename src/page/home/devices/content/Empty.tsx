import { Dns } from '@mui/icons-material';
import { Box, Typography } from '@mui/material';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { usePlatformMobile } from '@/common/context/usePlatformMobile';
import { MobileHeaderAction } from '@/page/home/components/MobileHeader.tsx';

export function Empty(props: { switchVisible: () => void }) {
  const { switchVisible } = props;
  const { t } = useI18n();
  const mobileViewport = usePlatformMobile();

  return (
    <Box className="tm-empty-state tm-mobile-empty">
      <Box className="tm-empty-state-card">
        <Box className="tm-empty-state-icon">
          <Dns className="tm-empty-state-icon-svg" />
        </Box>
        <Typography className="tm-empty-state-title" onClick={switchVisible}>
          {t('home.devices.emptyTitle')}
        </Typography>
        <Typography className="tm-empty-state-message">{t('home.devices.emptyMessage')}</Typography>
        {mobileViewport ? <MobileHeaderAction className="tm-empty-state-cta" label={t('home.devices.addHost')} onClick={switchVisible} /> : null}
      </Box>
    </Box>
  );
}
