import { Box, Typography } from '@mui/material';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { useVirtualizer } from '@tanstack/react-virtual';
import { isNil, isNotNil } from '@/common/util/StringUtil.ts';
import type { AiChatItem } from '@/common/model/AiChatItem.ts';
import type { ItemHandle } from '@/page/common/dto/agent/ItemHandle.ts';
import { resolveTurnReqId } from '@/page/common/util/agentRunSummary.ts';
import { useShellDaemons } from '@/page/common/hooks/useShellDaemons.ts';
import { ItemContextGroup } from '@/page/agent/main/body/ItemContextGroup.tsx';
import type { AgentQuestionAnswer } from '@/common/model/req/event/WsAgent.ts';
import { AgentTurnTail } from '@/page/agent/main/body/message/AgentTurnTail.tsx';
import { AgentRunStatus } from '@/page/agent/main/body/message/AgentRunStatus.tsx';
import { ItemTaskGroup } from '@/page/agent/main/body/tool/task/ItemTaskGroup.tsx';
import type { AgentRunSummaryBuffer } from '@/page/common/util/agentRunSummary.ts';
import { AgentDayDivider } from '@/page/agent/main/body/message/AgentDayDivider.tsx';
import { AgentMessageItem } from '@/page/agent/main/body/message/AgentMessageItem.tsx';
import { AgentTurnProcess } from '@/page/agent/main/body/message/AgentTurnProcess.tsx';
import type { AgentTurnClock, AgentTurnTiming } from '@/page/common/util/agentTurnClock.ts';
import type { AgentCompactionInfo, AgentStatusInfo } from '@/common/model/resp/RespAgent.ts';
import { ReactNode, Ref, useCallback, useEffect, useImperativeHandle, useLayoutEffect, useRef, useState } from 'react';
import { buildAgentTurnText, type AgentMessageRow, type AgentTurnCopyBuffer } from '@/page/common/util/agentTurnCopy.ts';
import { measureVirtualRow, observeAttachedElementRect, reconcileVirtualRows } from '@/page/common/util/virtualizerRect.ts';

const ROW_ESTIMATE_HEIGHT = 48;

function shouldAdjustScrollPositionOnItemSizeChange(): boolean {
  return false;
}

export interface MessageBodyViewHandle {
  markStick: () => void;
}

interface MessageBodyViewProps {
  loading: boolean;
  runDone: boolean;
  sessionId: string;
  turnTotal: number;
  summaryTick: number;
  chatId: string | null;
  rows: AgentMessageRow[];
  turnClock: AgentTurnClock;
  revertItemId?: string | null;
  lastContentKey: string | null;
  status: AgentStatusInfo | null;
  copyBuffer: AgentTurnCopyBuffer;
  ref?: Ref<MessageBodyViewHandle>;
  summaryBuffer: AgentRunSummaryBuffer;
  compaction: AgentCompactionInfo | null;
  onAskAnswer?: (answer: AgentQuestionAnswer) => void;
  onRevertEdit?: (itemId: string, message: string) => void;
  onRegisterHandle: (id: string, handle: ItemHandle | null) => void;
}

export function MessageBodyView(props: MessageBodyViewProps) {
  const { t } = useI18n();
  const { loading, runDone, sessionId, chatId, turnTotal, summaryTick, rows, status, compaction, revertItemId, lastContentKey, copyBuffer, summaryBuffer, turnClock, onAskAnswer, onRevertEdit, onRegisterHandle, ref } = props;

  const shouldStickRef = useRef(true);
  const scrollRef = useRef<HTMLDivElement | null>(null);
  const contentRef = useRef<HTMLDivElement | null>(null);

  const [contentReady, setContentReady] = useState(false);
  const [copiedTurn, setCopiedTurn] = useState<string | null>(null);
  const daemons = useShellDaemons(chatId);

  const rowVirtualizer = useVirtualizer({
    overscan: 6,
    count: rows.length,
    measureElement: measureVirtualRow,
    estimateSize: () => ROW_ESTIMATE_HEIGHT,
    getScrollElement: () => scrollRef.current,
    observeElementRect: observeAttachedElementRect,
    getItemKey: (index: number) => rows[index].key,
  });
  rowVirtualizer.shouldAdjustScrollPositionOnItemSizeChange = shouldAdjustScrollPositionOnItemSizeChange;
  const showLoading = loading || !contentReady;
  const revertRowIndex = isNotNil(revertItemId) ? rows.findIndex(row => isNotNil(row.item) && row.item.id === revertItemId) : -1;

  function isNearBottom(): boolean {
    const el = scrollRef.current;
    if (!el) {
      return true;
    }
    return el.scrollHeight - el.scrollTop - el.clientHeight <= 80;
  }

  function handleScroll(): void {
    shouldStickRef.current = isNearBottom();
  }

  useLayoutEffect(() => {
    reconcileVirtualRows(rowVirtualizer);
  });

  useImperativeHandle(ref, () => ({
    markStick: () => {
      shouldStickRef.current = true;
    },
  }));

  const handleCopyTurn = useCallback(
    (turn: AiChatItem[]): void => {
      const text = buildAgentTurnText(turn, copyBuffer);
      if (!text) {
        return;
      }
      navigator.clipboard
        .writeText(text)
        .then(() => {
          setCopiedTurn(turn[0].id);
        })
        .catch(err => {
          console.error('[agent] copy conversation error', err);
        });
    },
    [copyBuffer],
  );

  useEffect(() => {
    if (isNil(copiedTurn)) {
      return;
    }
    const timer = window.setTimeout(() => {
      setCopiedTurn(null);
    }, 1500);
    return () => {
      window.clearTimeout(timer);
    };
  }, [copiedTurn]);

  useEffect(() => {
    if (loading) {
      setContentReady(false);
      return;
    }
    shouldStickRef.current = true;
    let cancelled = false;
    let revealTimer = 0;
    const frame = window.requestAnimationFrame(() => {
      rowVirtualizer.scrollToEnd();
      revealTimer = window.setTimeout(() => {
        if (!cancelled) {
          setContentReady(true);
        }
      }, 48);
    });
    return () => {
      cancelled = true;
      window.cancelAnimationFrame(frame);
      window.clearTimeout(revealTimer);
    };
  }, [loading]);

  function resolveTurnTiming(row: AgentMessageRow): AgentTurnTiming | null {
    const reqId = resolveTurnReqId(row.turn);
    if (!reqId) {
      return null;
    }
    return turnClock.resolve(reqId);
  }

  function renderRow(row: AgentMessageRow): ReactNode {
    if (isNotNil(row.dayDivider)) {
      return <AgentDayDivider at={row.dayDivider} />;
    }
    if (isNotNil(row.processRows)) {
      return <AgentTurnProcess rows={row.processRows} timing={resolveTurnTiming(row)} backgroundCount={row.turnIndex === turnTotal - 1 ? daemons.length : 0} renderRow={renderRow} />;
    }
    if (isNotNil(row.groupItems)) {
      return <ItemContextGroup items={row.groupItems} active={!runDone && row.key === lastContentKey} sessionId={sessionId} onRegisterHandle={onRegisterHandle} />;
    }
    if (isNotNil(row.writeItems)) {
      return <ItemContextGroup items={row.writeItems} variant="write" active={!runDone && row.key === lastContentKey} sessionId={sessionId} onAskAnswer={onAskAnswer} onRegisterHandle={onRegisterHandle} />;
    }
    if (isNotNil(row.taskItems)) {
      return <ItemTaskGroup items={row.taskItems} sessionId={sessionId} onRegisterHandle={onRegisterHandle} />;
    }
    if (isNil(row.item)) {
      return <AgentTurnTail turn={row.turn} runDone={runDone} copiedTurn={copiedTurn} turnTotal={turnTotal} turnIndex={row.turnIndex} summaryTick={summaryTick} onCopyTurn={handleCopyTurn} copyBuffer={copyBuffer} summaryBuffer={summaryBuffer} />;
    }
    return <AgentMessageItem item={row.item} sessionId={sessionId} canEdit={runDone} editing={isNotNil(revertItemId) && row.item.id === revertItemId} onAskAnswer={onAskAnswer} onRevertEdit={onRevertEdit} onRegisterHandle={onRegisterHandle} />;
  }

  useEffect(() => {
    if (loading || !contentReady) {
      return;
    }
    const inner = contentRef.current;
    if (!inner) {
      return;
    }
    let lastHeight = 0;
    let scheduled = 0;
    const observer = new ResizeObserver((entries: ResizeObserverEntry[]) => {
      const height = Math.round(entries[0].contentRect.height);
      if (height <= lastHeight) {
        lastHeight = height;
        return;
      }
      lastHeight = height;
      if (scheduled !== 0) {
        return;
      }
      scheduled = window.requestAnimationFrame(() => {
        scheduled = 0;
        if (shouldStickRef.current) {
          rowVirtualizer.scrollToEnd();
        }
      });
    });
    observer.observe(inner);
    return () => {
      if (scheduled !== 0) {
        window.cancelAnimationFrame(scheduled);
      }
      observer.disconnect();
    };
  }, [loading, contentReady]);

  return (
    <Box className="tm-agent-message-wrap">
      <Box ref={scrollRef} className={showLoading ? 'tm-agent-message-body is-loading' : 'tm-agent-message-body'} data-tm-message-body="true" onScroll={handleScroll}>
        {showLoading ? <Typography className="tm-agent-message-loading">{t('agent.message.loading')}</Typography> : null}
        {loading ? null : (
          <Box ref={contentRef} className={contentReady ? 'tm-agent-message-content' : 'tm-agent-message-content is-pending'} data-tm-message-content="true" style={{ height: rowVirtualizer.getTotalSize() }}>
            {rowVirtualizer.getVirtualItems().map(virtualRow => {
              const row = rows[virtualRow.index];
              const dimmed = revertRowIndex >= 0 && virtualRow.index > revertRowIndex;
              return (
                <Box key={virtualRow.key} data-index={virtualRow.index} ref={rowVirtualizer.measureElement} className={dimmed ? 'tm-agent-msg-row is-revert-dim' : 'tm-agent-msg-row'} style={{ transform: `translateY(${virtualRow.start}px)` }}>
                  {renderRow(row)}
                </Box>
              );
            })}
          </Box>
        )}
      </Box>
      <AgentRunStatus visible={!showLoading} status={status} compaction={compaction} />
    </Box>
  );
}
