import { useState } from 'react';
import { Box, Typography } from '@mui/material';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { APP_THEME_OPTIONS } from '@/common/model/AppTheme.ts';
import { ThemeField } from '@/page/common/components/input/ThemeField.tsx';
import { useTerminalThemeService } from '@/common/context/useAppHooks.tsx';

export function ThemeRow() {
  const { t } = useI18n();
  const terminalThemeService = useTerminalThemeService();
  const [themeId, setThemeId] = useState<string>(() => terminalThemeService.getAppThemeId());
  const themes = APP_THEME_OPTIONS.map(theme => ({ ...theme, name: t(theme.nameKey), subtitleKey: theme.id === 'default' ? 'dark' : 'light' }));

  const handleChange = (value: string): void => {
    setThemeId(value);
    terminalThemeService.setAppThemeId(value);
  };

  return (
    <Box className="tm-set-row tm-set-theme-row">
      <Box className="tm-set-row-main">
        <Typography className="tm-set-row-label">{t('set.theme.title')}</Typography>
      </Box>
      <Box className="tm-set-row-control tm-set-theme-control">
        <ThemeField value={themeId} onChange={handleChange} themes={themes} compact />
      </Box>
    </Box>
  );
}
