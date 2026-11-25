import { memo } from 'react';
import type { ReactNode } from 'react';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { Box, ButtonBase, Tooltip } from '@mui/material';
import { Check, ContentCopy } from '@mui/icons-material';
import type { AiChatItem } from '@/common/model/AiChatItem.ts';
import { ItemRunSummary } from '@/page/agent/main/body/ItemRunSummary.tsx';
import { buildAgentTurnText, canCopyAgentTurn, type AgentTurnCopyBuffer } from '@/page/common/util/agentTurnCopy.ts';
import { canShowRunSummary, resolveTurnReqId, type AgentRunSummaryBuffer } from '@/page/common/util/agentRunSummary.ts';

interface AgentTurnTailProps {
  turn: AiChatItem[];
  turnIndex: number;
  turnTotal: number;
  runDone: boolean;
  summaryTick: number;
  copiedTurn: string | null;
  copyBuffer: AgentTurnCopyBuffer;
  summaryBuffer: AgentRunSummaryBuffer;
  onCopyTurn: (turn: AiChatItem[]) => void;
}

export const AgentTurnTail = memo(function AgentTurnTail(props: AgentTurnTailProps) {
  const { t } = useI18n();
  const { turn, turnIndex, turnTotal, runDone, summaryTick, copiedTurn, copyBuffer, summaryBuffer, onCopyTurn } = props;
  void summaryTick;

  function renderTurnCopy(): ReactNode {
    if (!canCopyAgentTurn(turnIndex, turnTotal, runDone)) {
      return null;
    }
    if (buildAgentTurnText(turn, copyBuffer).length === 0) {
      return null;
    }
    const copied = copiedTurn === turn[0].id;
    return (
      <Tooltip title={copied ? t('agent.copy.copied') : t('agent.copy.action')}>
        <ButtonBase disableRipple onClick={() => onCopyTurn(turn)} aria-label={t('agent.copy.action')} className="tm-agent-turn-copy">
          {copied ? <Check sx={{ fontSize: 16 }} /> : <ContentCopy sx={{ fontSize: 16 }} />}
        </ButtonBase>
      </Tooltip>
    );
  }

  function renderTurnSummary(): ReactNode {
    if (!canShowRunSummary(turnIndex, turnTotal, runDone)) {
      return null;
    }
    const reqId = resolveTurnReqId(turn);
    if (!reqId) {
      return null;
    }
    const lines = summaryBuffer.buildLines(reqId);
    if (lines.length === 0) {
      return null;
    }
    return <ItemRunSummary lines={lines} chatId={turn[0].chat_id} />;
  }

  function renderTurnFoot(): ReactNode {
    const copy = renderTurnCopy();
    if (!copy) {
      return null;
    }
    return (
      <Box className="tm-agent-turn-foot">
        <Box className="tm-agent-turn-actions">{copy}</Box>
      </Box>
    );
  }

  return (
    <Box className="tm-agent-turn-tail">
      {renderTurnSummary()}
      {renderTurnFoot()}
    </Box>
  );
});
