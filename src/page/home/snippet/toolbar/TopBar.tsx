import { Add } from '@mui/icons-material';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { Box, ButtonBase, Typography } from '@mui/material';
import { SearchBar } from '@/page/home/components/SearchBar.tsx';
import { usePlatformMobile } from '@/common/context/usePlatformMobile';
import { MobileHeader, MobileHeaderAction } from '@/page/home/components/MobileHeader.tsx';

export interface TopBarProps {
  onAdd: () => void;
  searchText: string | null;
  onSearchChange: (value: string | null) => void;
}

export function TopBar(props: TopBarProps) {
  const { t } = useI18n();
  const mobileViewport = usePlatformMobile();
  const { searchText, onSearchChange, onAdd } = props;

  if (mobileViewport) {
    return (
      <Box className="tm-home-topbar">
        <MobileHeader title={t('home.menu.snippets')} action={<MobileHeaderAction label={t('home.snippet.add')} onClick={onAdd} />} />
        <SearchBar placeholder={t('home.snippet.searchPlaceholder')} searchText={searchText} onChange={onSearchChange} />
      </Box>
    );
  }

  return (
    <>
      <SearchBar placeholder={t('home.snippet.searchPlaceholder')} searchText={searchText} onChange={onSearchChange} />
      <Box className="tm-agent-section-head">
        <Typography className="tm-agent-section-title">{t('home.menu.snippets')}</Typography>
        <ButtonBase disableRipple className="tm-agent-section-action" aria-label={t('home.snippet.add')} title={t('home.snippet.add')} onClick={onAdd}>
          <Add sx={{ fontSize: 18 }} />
        </ButtonBase>
      </Box>
    </>
  );
}
