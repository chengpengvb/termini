import { Box } from '@mui/material';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { Block, Check, ErrorOutline } from '@mui/icons-material';
import { formatDuration, formatFullDateTime } from '@/page/common/util/agentTimeFormat.ts';
import { TURN_END_STATE, type AgentTurnTiming } from '@/page/common/util/agentTurnClock.ts';

interface AgentTurnMetaProps {
  timing: AgentTurnTiming;
  backgroundCount?: number;
}

const STATE_I18N: Record<number, string> = {
  [TURN_END_STATE.DONE]: 'agent.turn.done',
  [TURN_END_STATE.CANCELLED]: 'agent.turn.cancelled',
  [TURN_END_STATE.FAILED]: 'agent.turn.failed',
};

const STATE_CLASS: Record<number, string> = {
  [TURN_END_STATE.DONE]: 'tm-agent-turn-meta is-done',
  [TURN_END_STATE.CANCELLED]: 'tm-agent-turn-meta is-cancelled',
  [TURN_END_STATE.FAILED]: 'tm-agent-turn-meta is-failed',
};

function renderIcon(state: number) {
  if (state === TURN_END_STATE.CANCELLED) {
    return <Block sx={{ fontSize: 13 }} />;
  }
  if (state === TURN_END_STATE.FAILED) {
    return <ErrorOutline sx={{ fontSize: 13 }} />;
  }
  return <Check sx={{ fontSize: 13 }} />;
}

export function AgentTurnMeta(props: AgentTurnMetaProps) {
  const { t, tf } = useI18n();
  const { timing, backgroundCount = 0 } = props;
  const duration = formatDuration(timing.endAt - timing.startAt);
  const title = tf('agent.turn.durationTitle', { start: formatFullDateTime(timing.startAt), end: formatFullDateTime(timing.endAt), duration });

  function renderBackground() {
    if (backgroundCount <= 0) {
      return null;
    }
    const text = backgroundCount === 1 ? t('agent.turn.backgroundOne') : tf('agent.turn.backgroundMany', { n: backgroundCount });
    return (
      <>
        <Box component="span" className="tm-agent-turn-meta-dot">
          ·
        </Box>
        <Box component="span" className="tm-agent-turn-meta-background">
          {text}
        </Box>
      </>
    );
  }

  return (
    <Box className={timing.approximate ? `${STATE_CLASS[timing.state]} is-approximate` : STATE_CLASS[timing.state]} title={title}>
      <Box component="span" className="tm-agent-turn-meta-icon">
        {renderIcon(timing.state)}
      </Box>
      <Box component="span" className="tm-agent-turn-meta-label">
        {t(STATE_I18N[timing.state])}
      </Box>
      <Box component="span" className="tm-agent-turn-meta-dot">
        ·
      </Box>
      <Box component="span" className="tm-agent-turn-meta-duration">
        {duration}
      </Box>
      {renderBackground()}
    </Box>
  );
}
