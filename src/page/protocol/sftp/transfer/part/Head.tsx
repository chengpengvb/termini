import { RefObject } from 'react';
import { Box, ButtonBase } from '@mui/material';
import Search from '@mui/icons-material/Search';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { ReqDevice } from '@/common/model/req/ReqDevice.ts';
import { SftpHandle } from '@/page/common/dto/sftp/SftpHandle.ts';
import { usePlatformMobile } from '@/common/context/usePlatformMobile';
import { HeadFilter } from '@/page/protocol/sftp/transfer/part/head/HeadFilter.tsx';
import { HeadActions } from '@/page/protocol/sftp/transfer/part/head/HeadActions.tsx';
import { HeadIdentity } from '@/page/protocol/sftp/transfer/part/head/HeadIdentity.tsx';

export interface HeadProps {
  sessionId: string;
  filterResetToken: number;
  reqDevice: ReqDevice | null;
  onIdentityClick: () => void;
  panelRef: RefObject<SftpHandle | null>;
}

export function Head(props: HeadProps) {
  const { t } = useI18n();
  const mobileViewport = usePlatformMobile();
  const { reqDevice, panelRef, filterResetToken, onIdentityClick } = props;

  const handleOpenSearch = (): void => {
    panelRef.current?.openSearch();
  };

  if (!reqDevice) {
    return null;
  }

  return (
    <Box className="tm-sftp-head">
      <HeadIdentity reqDevice={reqDevice} onClick={onIdentityClick} />
      <Box className="tm-sftp-head-tools">
        <HeadFilter panelRef={panelRef} filterResetToken={filterResetToken} />
        {mobileViewport ? (
          <ButtonBase disableRipple className="tm-sftp-head-search-btn" onClick={handleOpenSearch} aria-label={t('common.search')}>
            <Search className="tm-sftp-head-search-icon" />
          </ButtonBase>
        ) : null}
        <HeadActions panelRef={panelRef} />
      </Box>
    </Box>
  );
}
