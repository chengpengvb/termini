import { useMemo } from 'react';
import { Box, Typography } from '@mui/material';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { isNil, isNotNil } from '@/common/util/StringUtil.ts';
import { useElapsedMs } from '@/page/common/hooks/useElapsed.ts';
import type { AgentCompactionInfo, AgentStatusInfo } from '@/common/model/resp/RespAgent.ts';

const STATUS_STAGE_I18N: Record<string, string> = {
  routing: 'agent.message.routing',
  thinking: 'agent.message.thinking',
  memoryTrim: 'agent.message.memoryTrim',
  retrying: 'agent.message.retrying',
};

const ELAPSED_SHOW_MS = 3_000;

function formatTokens(value: number): string {
  if (value >= 1_000_000) {
    return `${(value / 1_000_000).toFixed(1)}M`;
  }
  if (value >= 1_000) {
    return `${(value / 1_000).toFixed(1)}K`;
  }
  return `${value}`;
}

interface AgentRunStatusProps {
  visible: boolean;
  status: AgentStatusInfo | null;
  compaction: AgentCompactionInfo | null;
}

export function AgentRunStatus(props: AgentRunStatusProps) {
  const { t, tf } = useI18n();
  const { visible, status, compaction } = props;
  const stage = status?.stage ?? null;
  const active = visible && stage !== null;
  const key = stage === null ? undefined : STATUS_STAGE_I18N[stage];
  const startedAt = useMemo(() => (active && stage !== null ? Date.now() : null), [active, stage]);
  const elapsedMs = useElapsedMs(startedAt, active);

  function resolveLabel(labelKey: string): string {
    if (stage === 'retrying' && isNotNil(status) && isNotNil(status.attempt) && isNotNil(status.maxAttempts)) {
      return tf(labelKey, { n: status.attempt, m: status.maxAttempts });
    }
    return t(labelKey);
  }

  function resolveText(labelKey: string): string {
    const label = resolveLabel(labelKey);
    if (isNil(elapsedMs) || elapsedMs < ELAPSED_SHOW_MS) {
      return label;
    }
    return `${label} ${Math.floor(elapsedMs / 1000)}s`;
  }

  if (!active || key === undefined) {
    return null;
  }

  return (
    <Box className="tm-agent-run-status">
      {isNotNil(compaction) ? <Typography className="tm-agent-run-status-note">{tf('agent.message.memoryTrimmed', { before: formatTokens(compaction.tokensBefore), after: formatTokens(compaction.tokensAfter) })}</Typography> : null}
      <Typography className="tm-agent-run-status-text tm-agent-shimmer">{resolveText(key)}</Typography>
    </Box>
  );
}
