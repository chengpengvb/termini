import { Actions } from './Actions';
import { Box } from '@mui/material';
import { useI18n } from '@/common/i18n/useI18n.ts';
import type { ReqDevice } from '@/common/model/req/ReqDevice.ts';
import { HostSearch } from '@/page/home/components/HostSearch.tsx';
import { usePlatformMobile } from '@/common/context/usePlatformMobile';
import { MobileHeader, MobileHeaderAction } from '@/page/home/components/MobileHeader.tsx';

export interface TopBarProps {
  viewMode: number;
  onAddHost: () => void;
  onImport: (source: number) => void;
  onQueryChange: (query: string) => void;
  onConfirm: (reqDevice: ReqDevice) => void;
  onViewModeChange: (mode: number) => void;
}

export function TopBar(props: TopBarProps) {
  const { viewMode, onAddHost, onImport, onQueryChange, onConfirm, onViewModeChange } = props;
  const { t } = useI18n();
  const mobileViewport = usePlatformMobile();

  const renderSearch = () => {
    return (
      <HostSearch
        placeholder={t('home.devices.searchPlaceholder')}
        barClassName="tm-home-searchbar"
        iconClassName="tm-home-search-icon"
        inputClassName="tm-home-search-input-root"
        inputInnerClassName="tm-home-search-input"
        confirmLabel={t('home.common.connect')}
        onConfirm={onConfirm}
        onQueryChange={onQueryChange}
      />
    );
  };

  if (mobileViewport) {
    return (
      <Box className="tm-home-topbar">
        <MobileHeader title={t('home.menu.connections')} action={<MobileHeaderAction label={t('home.devices.addHost')} onClick={onAddHost} />} />
        {renderSearch()}
      </Box>
    );
  }

  return (
    <Box className="tm-home-topbar">
      {renderSearch()}
      <Actions viewMode={viewMode} onChange={onViewModeChange} onAddHost={onAddHost} onImport={onImport} />
    </Box>
  );
}
