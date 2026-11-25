import type { ReactNode } from 'react';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { Add, ChevronLeft } from '@mui/icons-material';
import { Box, ButtonBase, Typography } from '@mui/material';

export interface MobileHeaderActionProps {
  label: string;
  onClick: () => void;
  className?: string;
}

export interface MobileHeaderProps {
  title: string;
  action?: ReactNode;
  leading?: ReactNode;
  onBack?: () => void;
}

export function MobileHeaderAction(props: MobileHeaderActionProps) {
  const { t } = useI18n();
  const { label, onClick, className } = props;

  const resolveClassName = (): string => {
    if (className == null) {
      return 'tm-mobile-header-action';
    }
    return `tm-mobile-header-action ${className}`;
  };

  return (
    <ButtonBase disableRipple className={resolveClassName()} onClick={onClick} aria-label={label}>
      <Add className="tm-mobile-header-action-icon" />
      <Box component="span" className="tm-mobile-header-action-label">
        {t('common.add')}
      </Box>
    </ButtonBase>
  );
}

export function MobileHeader(props: MobileHeaderProps) {
  const { t } = useI18n();
  const { title, action, leading, onBack } = props;

  const resolveClassName = (): string => {
    if (onBack) {
      return 'tm-mobile-header has-back';
    }
    return 'tm-mobile-header';
  };

  const renderLeading = (): ReactNode => {
    if (onBack) {
      return (
        <ButtonBase disableRipple className="tm-mobile-header-back" onClick={onBack} aria-label={t('common.back')}>
          <ChevronLeft className="tm-mobile-header-back-icon" />
        </ButtonBase>
      );
    }
    return leading;
  };

  return (
    <Box className={resolveClassName()}>
      <Box className="tm-mobile-header-side">{renderLeading()}</Box>
      <Typography component="h1" className="tm-mobile-header-title">
        {title}
      </Typography>
      <Box className="tm-mobile-header-side is-end">{action}</Box>
    </Box>
  );
}
