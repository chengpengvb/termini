import { Actions } from './Actions';
import { Box } from '@mui/material';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { SearchBar } from '@/page/home/components/SearchBar.tsx';
import { usePlatformMobile } from '@/common/context/usePlatformMobile';
import { MobileHeader, MobileHeaderAction } from '@/page/home/components/MobileHeader.tsx';

export interface TopBarProps {
  viewMode: number;
  onAddKey: () => void;
  searchText: string | null;
  onViewModeChange: (mode: number) => void;
  onSearchChange: (value: string | null) => void;
}

export function TopBar(props: TopBarProps) {
  const { searchText, onSearchChange, viewMode, onViewModeChange, onAddKey } = props;
  const { t } = useI18n();
  const mobileViewport = usePlatformMobile();

  if (mobileViewport) {
    return (
      <Box className="tm-home-topbar">
        <MobileHeader title={t('home.menu.keys')} action={<MobileHeaderAction label={t('home.keys.addKey')} onClick={onAddKey} />} />
        <SearchBar placeholder={t('home.keys.searchPlaceholder')} searchText={searchText} onChange={onSearchChange} />
      </Box>
    );
  }

  return (
    <Box className="tm-home-topbar">
      <SearchBar placeholder={t('home.keys.searchPlaceholder')} searchText={searchText} onChange={onSearchChange} />
      <Actions viewMode={viewMode} onChange={onViewModeChange} onAddKey={onAddKey} />
    </Box>
  );
}
