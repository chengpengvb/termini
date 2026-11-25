import { useI18n } from '@/common/i18n/useI18n.ts';
import { Box, ButtonBase, Popover } from '@mui/material';
import { isNil, isNotNil } from '@/common/util/StringUtil.ts';
import type { AiChatItem } from '@/common/model/AiChatItem.ts';
import { useCallback, useEffect, useRef, useState } from 'react';
import type { ContextUsage } from '@/common/model/dto/ContextUsage.ts';
import { OverlayPopoverPaper } from '@/page/common/util/scopedDialogUtil.ts';
import type { RespSocketContext } from '@/common/model/resp/RespWsContext.ts';
import { RESP_EVENT_AGENT } from '@/common/model/resp/event/RespEventAgent.ts';
import { useAgentService, useAiChatItemService } from '@/common/context/useAppHooks.tsx';

interface ContextUsageBadgeProps {
  running: boolean;
  chatId: string | null;
  provideId: string | null;
}

interface UsageSegment {
  labelKey: string;
  className: string;
  key: keyof Pick<ContextUsage, 'systemPrompt' | 'toolDefinitions' | 'mcpTools' | 'summary' | 'conversation'>;
}

const SEGMENTS: UsageSegment[] = [
  { key: 'systemPrompt', labelKey: 'agent.toolbar.usageSystemPrompt', className: 'is-system' },
  { key: 'toolDefinitions', labelKey: 'agent.toolbar.usageToolDefinitions', className: 'is-tools' },
  { key: 'mcpTools', labelKey: 'agent.toolbar.usageMcpTools', className: 'is-mcp' },
  { key: 'summary', labelKey: 'agent.toolbar.usageSummary', className: 'is-summary' },
  { key: 'conversation', labelKey: 'agent.toolbar.usageConversation', className: 'is-conversation' },
];

const RING_SIZE = 16;
const RING_STROKE = 2;
const RING_RADIUS = (RING_SIZE - RING_STROKE) / 2;
const RING_CIRCUMFERENCE = 2 * Math.PI * RING_RADIUS;

function formatTokens(value: number): string {
  if (value >= 1_000_000) {
    return `${(value / 1_000_000).toFixed(1)}M`;
  }
  if (value >= 1_000) {
    return `${(value / 1_000).toFixed(1)}K`;
  }
  return `${value}`;
}

function usedTokens(usage: ContextUsage): number {
  return usage.measured > 0 ? usage.measured : usage.total;
}

function percentOf(usage: ContextUsage): number | null {
  if (usage.tokenLimit <= 0) {
    return null;
  }
  return Math.round((usedTokens(usage) / usage.tokenLimit) * 100);
}

function isProgressEvent(type: number): boolean {
  return type === RESP_EVENT_AGENT.ON_STATUS || type === RESP_EVENT_AGENT.ON_TOOL_CALL_DONE || type === RESP_EVENT_AGENT.ON_TEXT_FINAL;
}

export function ContextUsageBadge(props: ContextUsageBadgeProps) {
  const { t, tf } = useI18n();
  const requestRef = useRef(0);
  const { running, chatId, provideId } = props;
  const agentService = useAgentService();
  const aiChatItemService = useAiChatItemService();
  const [usage, setUsage] = useState<ContextUsage | null>(null);
  const [anchor, setAnchor] = useState<HTMLElement | null>(null);

  const refresh = useCallback(async (): Promise<void> => {
    if (isNil(chatId)) {
      setUsage(null);
      return;
    }
    const ticket = ++requestRef.current;
    try {
      const next = await aiChatItemService.contextUsage(chatId, provideId);
      if (ticket === requestRef.current) {
        setUsage(next);
      }
    } catch (err) {
      console.error('[agent] load context usage error', err);
    }
  }, [aiChatItemService, chatId, provideId]);

  useEffect(() => {
    void refresh();
  }, [refresh, running]);

  useEffect(() => {
    if (isNil(chatId)) {
      return;
    }
    return agentService.addEventListener(chatId, (arg: { context: RespSocketContext }) => {
      const item = arg.context.payload as AiChatItem;
      if (!item || !isProgressEvent(item.type)) {
        return;
      }
      void refresh();
    });
  }, [agentService, chatId, refresh]);

  function handleOpen(event: React.MouseEvent<HTMLElement>): void {
    setAnchor(event.currentTarget);
    void refresh();
  }

  function handleClose(): void {
    setAnchor(null);
  }

  if (isNil(chatId)) {
    return null;
  }

  const percent = isNotNil(usage) ? percentOf(usage) : null;
  const ratio = isNotNil(percent) ? Math.min(percent, 100) / 100 : 0;
  const ringClassName = isNotNil(percent) && percent >= 100 ? 'tm-agent-context-ring is-over' : isNotNil(percent) && percent >= 80 ? 'tm-agent-context-ring is-high' : 'tm-agent-context-ring';
  const label = isNotNil(usage) ? (isNotNil(percent) ? tf('agent.toolbar.usageFull', { percent }) : formatTokens(usedTokens(usage))) : t('agent.toolbar.usageTitle');

  function renderTotal(current: ContextUsage): string {
    const used = formatTokens(usedTokens(current));
    const prefix = current.measured > 0 ? '' : '~';
    if (isNotNil(percent)) {
      return `${prefix}${used} / ${formatTokens(current.tokenLimit)} Tokens`;
    }
    return `${prefix}${used} Tokens`;
  }

  return (
    <>
      <ButtonBase disableRipple onClick={handleOpen} aria-label={t('agent.toolbar.usageTitle')} title={label} className={ringClassName}>
        <svg width={RING_SIZE} height={RING_SIZE} viewBox={`0 0 ${RING_SIZE} ${RING_SIZE}`} aria-hidden="true">
          <circle className="tm-agent-context-ring-track" cx={RING_SIZE / 2} cy={RING_SIZE / 2} r={RING_RADIUS} strokeWidth={RING_STROKE} />
          <circle className="tm-agent-context-ring-fill" cx={RING_SIZE / 2} cy={RING_SIZE / 2} r={RING_RADIUS} strokeWidth={RING_STROKE} strokeDasharray={`${RING_CIRCUMFERENCE * ratio} ${RING_CIRCUMFERENCE}`} transform={`rotate(-90 ${RING_SIZE / 2} ${RING_SIZE / 2})`} />
        </svg>
      </ButtonBase>
      <Popover
        open={isNotNil(anchor)}
        anchorEl={anchor}
        onClose={handleClose}
        anchorOrigin={{ vertical: 'top', horizontal: 'right' }}
        transformOrigin={{ vertical: 'bottom', horizontal: 'right' }}
        slots={{ paper: OverlayPopoverPaper }}
        slotProps={{ paper: { className: 'tm-agent-context-popover' } }}
      >
        <Box className="tm-agent-context-head">
          <Box component="span" className="tm-agent-context-title">
            {t('agent.toolbar.usageTitle')}
          </Box>
          <ButtonBase disableRipple onClick={handleClose} aria-label={t('common.close')} className="tm-agent-context-close">
            ×
          </ButtonBase>
        </Box>
        {isNil(usage) ? (
          <Box className="tm-agent-context-empty">{t('agent.toolbar.usageLoading')}</Box>
        ) : (
          <>
            <Box className="tm-agent-context-summary">
              <Box component="span" className="tm-agent-context-percent">
                {isNotNil(percent) ? tf('agent.toolbar.usageFull', { percent }) : t('agent.toolbar.usageNoLimit')}
              </Box>
              <Box component="span" className="tm-agent-context-total">
                {renderTotal(usage)}
              </Box>
            </Box>
            <Box className="tm-agent-context-note">{usage.measured > 0 ? t('agent.toolbar.usageMeasured') : t('agent.toolbar.usageEstimated')}</Box>
            <Box className="tm-agent-context-bar" aria-hidden="true">
              {SEGMENTS.map(segment => {
                const value = usage[segment.key];
                const base = usage.tokenLimit > 0 ? Math.max(usage.tokenLimit, usage.total) : usage.total;
                const width = base > 0 ? (value / base) * 100 : 0;
                return width > 0 ? <Box key={segment.key} className={`tm-agent-context-bar-seg ${segment.className}`} style={{ width: `${width}%` }} /> : null;
              })}
            </Box>
            <Box className="tm-agent-context-rows">
              {SEGMENTS.map(segment => (
                <Box key={segment.key} className="tm-agent-context-row">
                  <Box component="span" className={`tm-agent-context-swatch ${segment.className}`} />
                  <Box component="span" className="tm-agent-context-label">
                    {t(segment.labelKey)}
                  </Box>
                  <Box component="span" className="tm-agent-context-value">
                    {formatTokens(usage[segment.key])}
                  </Box>
                </Box>
              ))}
            </Box>
          </>
        )}
      </Popover>
    </>
  );
}
