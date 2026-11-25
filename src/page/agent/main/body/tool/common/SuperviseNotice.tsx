import { Box } from '@mui/material';
import type { ReactNode } from 'react';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { isNil } from '@/common/util/StringUtil.ts';
import { useElapsed } from '@/page/common/hooks/useElapsed.ts';
import { SUPERVISE_ACTION, type SuperviseRound } from '@/page/common/dto/agent/ToolView.ts';
import { HourglassEmptyOutlined, LayersOutlined, StopCircleOutlined } from '@mui/icons-material';

interface SuperviseNoticeProps {
  active: boolean;
  round: SuperviseRound | null;
  startedAt: number | null;
}

function resolveActionLabel(action: number, t: (key: string) => string): string {
  if (action === SUPERVISE_ACTION.KILL) {
    return t('agent.tool.superviseKill');
  }
  if (action === SUPERVISE_ACTION.DETACH) {
    return t('agent.tool.superviseDetach');
  }
  return t('agent.tool.superviseWait');
}

function resolveRowClassName(action: number, waiting: boolean): string {
  if (action === SUPERVISE_ACTION.KILL) {
    return 'tm-agent-tool-supervise-row tm-agent-tool-supervise-row-alert';
  }
  if (waiting) {
    return 'tm-agent-tool-supervise-row tm-agent-tool-supervise-row-active';
  }
  return 'tm-agent-tool-supervise-row';
}

function resolveActionIcon(action: number): ReactNode {
  if (action === SUPERVISE_ACTION.KILL) {
    return <StopCircleOutlined className="tm-agent-tool-supervise-icon" />;
  }
  if (action === SUPERVISE_ACTION.DETACH) {
    return <LayersOutlined className="tm-agent-tool-supervise-icon" />;
  }
  return <HourglassEmptyOutlined className="tm-agent-tool-supervise-icon" />;
}

export function SuperviseNotice(props: SuperviseNoticeProps) {
  const { t } = useI18n();
  const { round, active, startedAt } = props;
  const waiting = active && (isNil(round) || round.action === SUPERVISE_ACTION.WAIT);
  const elapsed = useElapsed(startedAt, waiting);

  if (isNil(round)) {
    return null;
  }

  const head = isNil(elapsed) ? resolveActionLabel(round.action, t) : `${resolveActionLabel(round.action, t)} · ${elapsed}`;

  return (
    <Box className="tm-agent-tool-supervise">
      <Box className={resolveRowClassName(round.action, waiting)}>
        {resolveActionIcon(round.action)}
        <Box className="tm-agent-tool-supervise-body">
          <Box component="span" className={waiting ? 'tm-agent-tool-supervise-head tm-agent-shimmer' : 'tm-agent-tool-supervise-head'}>
            {head}
          </Box>
          {round.reason ? (
            <Box component="span" className="tm-agent-tool-supervise-text">
              {round.reason}
            </Box>
          ) : null}
        </Box>
      </Box>
    </Box>
  );
}
