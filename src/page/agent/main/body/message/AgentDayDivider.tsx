import { memo } from 'react';
import { Box } from '@mui/material';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { formatDayLabel, formatFullDateTime } from '@/page/common/util/agentTimeFormat.ts';

interface AgentDayDividerProps {
  at: number;
}

export const AgentDayDivider = memo(function AgentDayDivider(props: AgentDayDividerProps) {
  const { at } = props;
  useI18n();
  return (
    <Box className="tm-agent-day-divider" role="separator" aria-label={formatFullDateTime(at)}>
      <Box component="span" className="tm-agent-day-divider-label" title={formatFullDateTime(at)}>
        {formatDayLabel(at)}
      </Box>
    </Box>
  );
});
