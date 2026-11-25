import { useRef, useState } from 'react';
import { DeleteSweep } from '@mui/icons-material';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { Box, ButtonBase, Typography } from '@mui/material';
import { SearchBar } from '@/page/home/components/SearchBar.tsx';
import { resolveErrorText } from '@/common/model/resp/ResultModel.ts';
import { useSessionRecordService } from '@/common/context/useAppHooks.tsx';
import { ConfirmDialog } from '@/common/components/dialog/ConfirmDialog.tsx';
import { CommonMenu, type CommonMenuPosition } from '@/common/components/menu/CommonMenu.tsx';

const DAY_MS = 86_400_000;

export interface TopBarProps {
  onCleaned: () => void;
  searchText: string | null;
  onSearchChange: (value: string | null) => void;
}

export function TopBar(props: TopBarProps) {
  const { t, tf } = useI18n();
  const { searchText, onSearchChange, onCleaned } = props;
  const sessionRecordService = useSessionRecordService();
  const cleanAnchorRef = useRef<HTMLDivElement | null>(null);
  const [pendingDays, setPendingDays] = useState<number | null>(null);
  const [confirmError, setConfirmError] = useState<string | null>(null);
  const [menuPosition, setMenuPosition] = useState<CommonMenuPosition | null>(null);

  const CLEAN_MENU = { DAYS_7: 1, DAYS_30: 2, ALL: 3 } as const;

  const handleOpenMenu = (): void => {
    const anchor = cleanAnchorRef.current;
    if (anchor === null) {
      return;
    }
    const rect = anchor.getBoundingClientRect();
    setMenuPosition({ top: rect.bottom + 4, left: rect.left });
  };

  const handleCloseMenu = (): void => {
    setMenuPosition(null);
  };

  const handlePickDays = (days: number): void => {
    setConfirmError(null);
    setPendingDays(days);
  };

  const handleCancelConfirm = (): void => {
    setPendingDays(null);
    setConfirmError(null);
  };

  const handleConfirmClean = async (): Promise<void> => {
    if (pendingDays === null) {
      return;
    }
    const cutoff = pendingDays === 0 ? Number.MAX_SAFE_INTEGER : Date.now() - pendingDays * DAY_MS;
    try {
      await sessionRecordService.deleteBefore(cutoff);
      setPendingDays(null);
      setConfirmError(null);
      onCleaned();
    } catch (err) {
      setConfirmError(resolveErrorText(err));
    }
  };

  const resolveConfirmMessage = (): string => {
    if (pendingDays === null) {
      return '';
    }
    if (pendingDays === 0) {
      return t('home.record.cleanConfirmAll');
    }
    return tf('home.record.cleanConfirmDays', { days: pendingDays });
  };

  return (
    <>
      <SearchBar placeholder={t('home.record.searchPlaceholder')} searchText={searchText} onChange={onSearchChange} />
      <Box className="tm-agent-section-head">
        <Typography className="tm-agent-section-title">{t('home.menu.records')}</Typography>
        <Box ref={cleanAnchorRef}>
          <ButtonBase disableRipple className="tm-agent-section-action" aria-label={t('home.record.clean')} title={t('home.record.clean')} onClick={handleOpenMenu}>
            <DeleteSweep sx={{ fontSize: 18 }} />
          </ButtonBase>
        </Box>
      </Box>
      <CommonMenu
        position={menuPosition}
        onClose={handleCloseMenu}
        items={[
          { key: CLEAN_MENU.DAYS_7, text: t('home.record.cleanDays7'), onClick: () => handlePickDays(7) },
          { key: CLEAN_MENU.DAYS_30, text: t('home.record.cleanDays30'), onClick: () => handlePickDays(30) },
          { key: CLEAN_MENU.ALL, text: t('home.record.cleanAll'), danger: true, onClick: () => handlePickDays(0) },
        ]}
      />
      <ConfirmDialog
        open={pendingDays !== null}
        danger
        title={t('home.record.cleanTitle')}
        message={resolveConfirmMessage()}
        confirmLabel={t('common.delete')}
        error={confirmError}
        onCancel={handleCancelConfirm}
        onConfirm={() => {
          void handleConfirmClean();
        }}
      />
    </>
  );
}
