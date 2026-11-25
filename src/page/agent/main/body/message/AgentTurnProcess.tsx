import { Box, ButtonBase } from '@mui/material';
import { useState, type ReactNode } from 'react';
import { useI18n } from '@/common/i18n/useI18n.ts';
import type { AgentMessageRow } from '@/page/common/util/agentTurnCopy.ts';
import { KeyboardArrowDown, KeyboardArrowRight } from '@mui/icons-material';
import type { AgentTurnTiming } from '@/page/common/util/agentTurnClock.ts';
import { AgentTurnMeta } from '@/page/agent/main/body/message/AgentTurnMeta.tsx';

interface AgentTurnProcessProps {
  rows: AgentMessageRow[];
  backgroundCount: number;
  timing: AgentTurnTiming | null;
  renderRow: (row: AgentMessageRow) => ReactNode;
}

export function AgentTurnProcess(props: AgentTurnProcessProps) {
  const { t, tf } = useI18n();
  const [open, setOpen] = useState(false);
  const { rows, timing, backgroundCount, renderRow } = props;

  function resolveStepsText(): string {
    if (rows.length === 1) {
      return t('agent.turn.processStepOne');
    }
    return tf('agent.turn.processStepMany', { n: rows.length });
  }

  function handleToggle(): void {
    setOpen(current => !current);
  }

  return (
    <Box className={open ? 'tm-agent-fold-row tm-agent-turn-process is-open' : 'tm-agent-fold-row tm-agent-turn-process'}>
      <ButtonBase disableRipple className="tm-agent-fold-toggle" onClick={handleToggle}>
        {timing ? <AgentTurnMeta timing={timing} backgroundCount={backgroundCount} /> : null}
        <Box component="span" className="tm-agent-turn-process-steps">
          {resolveStepsText()}
        </Box>
        <Box component="span" className="tm-agent-fold-chevron">
          {open ? <KeyboardArrowDown sx={{ fontSize: 16 }} /> : <KeyboardArrowRight sx={{ fontSize: 16 }} />}
        </Box>
      </ButtonBase>
      {open ? (
        <Box className="tm-agent-turn-process-body">
          {rows.map(row => (
            <Box key={row.key} className="tm-agent-turn-process-row">
              {renderRow(row)}
            </Box>
          ))}
        </Box>
      ) : null}
    </Box>
  );
}
