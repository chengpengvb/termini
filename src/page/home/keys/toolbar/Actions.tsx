import { Box } from '@mui/material';
import { VpnKey } from '@mui/icons-material';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { ViewMode } from '../../components/ViewMode';
import { ActionPill } from '../../components/ActionPill';

export interface ActionsProps {
  viewMode: number;
  onAddKey: () => void;
  onChange: (mode: number) => void;
}

export function Actions(props: ActionsProps) {
  const {t} = useI18n();
  const { viewMode, onChange, onAddKey } = props;

  return (
    <Box className="tm-home-actions-row">
      <ActionPill icon={<VpnKey />} label={t('home.keys.addKey')} primary onClick={onAddKey} />
      <Box className="tm-home-actions-spacer" />
      <ViewMode viewMode={viewMode} onChange={onChange} />
    </Box>
  );
}
