import { isNil, isNotNil } from '@/common/util/StringUtil.ts';
import type { AiChatItem } from '@/common/model/AiChatItem.ts';
import { AgentEventHub } from '@/page/common/util/AgentEventHub.ts';
import { AgentTurnClock } from '@/page/common/util/agentTurnClock.ts';
import type { ItemHandle } from '@/page/common/dto/agent/ItemHandle.ts';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { RespSocketContext } from '@/common/model/resp/RespWsContext.ts';
import { RESP_EVENT_AGENT } from '@/common/model/resp/event/RespEventAgent.ts';
import type { AgentQuestionAnswer } from '@/common/model/req/event/WsAgent.ts';
import { useRunOutcome } from '@/page/agent/main/body/message/run/useRunOutcome.ts';
import type { AgentCompactionInfo, AgentStatusInfo } from '@/common/model/resp/RespAgent.ts';
import { AgentRunSummaryBuffer, canShowRunSummary } from '@/page/common/util/agentRunSummary.ts';
import { useAgentService, useAiChatItemService, useDeviceService } from '@/common/context/useAppHooks.tsx';
import { AgentTurnCopyBuffer, flattenAgentRows, groupAgentTurns } from '@/page/common/util/agentTurnCopy.ts';
import { MessageBodyView, type MessageBodyViewHandle } from '@/page/agent/main/body/message/MessageBodyView.tsx';
import { buildAgentRoot, ingestAgentHistoryItems, type AgentRootTarget } from '@/page/common/util/agentMessageHistory.ts';

interface MessageBodyProps {
  sessionId: string;
  hideUser?: boolean;
  chatId: string | null;
  revertItemId?: string | null;
  onAskAnswer?: (answer: AgentQuestionAnswer) => void;
  onInterruptedWithoutOutput?: (message: string) => void;
  onRevertEdit?: (itemId: string, message: string) => void;
}

export function MessageBody(props: MessageBodyProps) {
  const readyRef = useRef(false);
  const agentService = useAgentService();
  const chatIdRef = useRef(props.chatId);
  const deviceService = useDeviceService();
  const hubRef = useRef(new AgentEventHub());
  const [runDone, setRunDone] = useState(true);
  const [loading, setLoading] = useState(true);
  const aiChatItemService = useAiChatItemService();
  const turnClockRef = useRef(new AgentTurnClock());
  const [summaryTick, setSummaryTick] = useState(0);
  const pendingIncomingRef = useRef<AiChatItem[]>([]);
  const copyBufferRef = useRef(new AgentTurnCopyBuffer());
  const viewRef = useRef<MessageBodyViewHandle | null>(null);
  const summaryBufferRef = useRef(new AgentRunSummaryBuffer());
  const [historyGeneration, setHistoryGeneration] = useState(0);
  const [aiChatItems, setAiChatItems] = useState<AiChatItem[]>([]);
  const [status, setStatus] = useState<AgentStatusInfo | null>(null);
  const [compaction, setCompaction] = useState<AgentCompactionInfo | null>(null);
  const { sessionId, chatId, hideUser, revertItemId, onAskAnswer, onRevertEdit, onInterruptedWithoutOutput } = props;
  const outcome = useRunOutcome({ onInterruptedWithoutOutput });

  chatIdRef.current = chatId;
  const turns = useMemo(() => groupAgentTurns(aiChatItems), [aiChatItems]);
  const rows = useMemo(() => {
    const flat = flattenAgentRows(turns, turnIndex => canShowRunSummary(turnIndex, turns.length, runDone));
    if (!hideUser) {
      return flat;
    }
    return flat.filter(row => isNil(row.item) || row.item.type !== RESP_EVENT_AGENT.ON_USER);
  }, [turns, hideUser, runDone]);
  const lastContentKey = useMemo(() => {
    for (let index = rows.length - 1; index >= 0; index--) {
      const row = rows[index];
      if (isNotNil(row.item) || isNotNil(row.groupItems) || isNotNil(row.writeItems) || isNotNil(row.taskItems) || isNotNil(row.processRows)) {
        return row.key;
      }
    }
    return null;
  }, [rows]);

  function bumpSummary(): void {
    setSummaryTick((current: number) => current + 1);
  }

  function canAcceptItem(item: AiChatItem): boolean {
    const currentChatId = chatIdRef.current;
    if (!currentChatId) {
      return false;
    }
    if (!item.chat_id) {
      return false;
    }
    return item.chat_id === currentChatId;
  }

  function isRunEndEvent(type: number): boolean {
    return type === RESP_EVENT_AGENT.ON_DONE || type === RESP_EVENT_AGENT.ON_CANCELLED;
  }

  function appendRoot(item: AiChatItem, target: AgentRootTarget): void {
    const hub = hubRef.current;
    if (hub.hasRoot(target.rootId)) {
      if (!target.self) {
        return;
      }
      setAiChatItems(prev =>
        prev.map(row => {
          if (row.id !== item.id) {
            return row;
          }
          if (row.type === item.type) {
            return row;
          }
          return { ...row, type: item.type, context: item.context, timestamp: item.timestamp };
        }),
      );
      return;
    }
    hub.addRoot(target.rootId);
    if (item.type === RESP_EVENT_AGENT.ON_USER) {
      viewRef.current?.markStick();
    }
    setAiChatItems(prev => [...prev, buildAgentRoot(item, target)]);
  }

  function applyHistoryIngest(items: AiChatItem[]): void {
    const ingested = ingestAgentHistoryItems(items, canAcceptItem, copyBufferRef.current, summaryBufferRef.current, turnClockRef.current);
    hubRef.current.applyIngest(ingested);
    setRunDone(ingested.runDone);
    setAiChatItems(ingested.roots);
    bumpSummary();
  }

  function applyStatus(item: AiChatItem): void {
    const stage: string = item.context.stage;
    if (stage === 'memoryTrimmed') {
      setCompaction({ tokensBefore: item.context.tokensBefore ?? 0, tokensAfter: item.context.tokensAfter ?? 0 });
      setStatus({ stage: 'thinking', attempt: null, maxAttempts: null });
      return;
    }
    setStatus({ stage, attempt: item.context.attempt ?? null, maxAttempts: item.context.maxAttempts ?? null });
  }

  function handleRunEnd(item: AiChatItem): void {
    if (hubRef.current.isNested(item)) {
      const nestedRootId = hubRef.current.nestedRootId(item);
      if (isNotNil(nestedRootId)) {
        hubRef.current.dispatch(nestedRootId, item);
      }
      return;
    }
    hubRef.current.addRunEnd(item.context.reqId, item);
    setRunDone(true);
    setStatus(null);
    hubRef.current.broadcast(item);
    outcome.finishRun(item.type);
  }

  function handleIncomingItem(item: AiChatItem): void {
    if (!canAcceptItem(item)) {
      return;
    }
    if (turnClockRef.current.accumulate(item)) {
      bumpSummary();
    }
    const nested = hubRef.current.isNested(item);
    if (!nested) {
      copyBufferRef.current.accumulate(item);
      if (summaryBufferRef.current.accumulate(item)) {
        bumpSummary();
      }
    }
    if (item.type === RESP_EVENT_AGENT.ON_CONFLICT && !item.context.autoAllowed) {
      outcome.requestAttention();
    }
    if (item.type === RESP_EVENT_AGENT.ON_STATUS) {
      if (!nested) {
        applyStatus(item);
      }
      return;
    }
    if (item.type === RESP_EVENT_AGENT.ON_OPEN) {
      outcome.resetRun();
      setCompaction(null);
      setRunDone(false);
      return;
    }
    if (isRunEndEvent(item.type)) {
      handleRunEnd(item);
      return;
    }
    const target = hubRef.current.resolve(item);
    if (isNil(target)) {
      return;
    }
    if (!nested) {
      outcome.trackRunOutput(item);
    }
    setStatus(null);
    appendRoot(item, target);
    hubRef.current.dispatch(target.rootId, item);
  }

  function acceptIncomingItem(item: AiChatItem): void {
    if (!readyRef.current) {
      pendingIncomingRef.current.push(item);
      return;
    }
    handleIncomingItem(item);
  }

  function clear(): void {
    readyRef.current = false;
    pendingIncomingRef.current = [];
    hubRef.current.clear();
    copyBufferRef.current.clear();
    summaryBufferRef.current.clear();
    turnClockRef.current.clear();
    setRunDone(true);
    setStatus(null);
    setCompaction(null);
    setSummaryTick(0);
    setAiChatItems([]);
  }

  const registerHandle = useCallback((id: string, handle: ItemHandle | null): void => {
    hubRef.current.register(id, handle);
  }, []);

  useEffect(() => {
    let cancelled = false;
    clear();
    setLoading(isNotNil(chatId));
    if (isNil(chatId)) {
      setLoading(false);
      return;
    }
    async function loadHistory(): Promise<void> {
      readyRef.current = false;
      try {
        await deviceService.ensureDeviceCache();
        if (cancelled) {
          return;
        }
        const items = await aiChatItemService.selectByChatId(chatId as string);
        if (cancelled) {
          return;
        }
        applyHistoryIngest(items);
        readyRef.current = true;
        const pending = pendingIncomingRef.current;
        pendingIncomingRef.current = [];
        for (const item of pending) {
          handleIncomingItem(item);
        }
      } catch (err) {
        console.error('[agent] load chat items error', err);
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    }
    void loadHistory();
    const offResync = agentService.addResyncListener(chatId, () => {
      clear();
      setHistoryGeneration((current: number) => current + 1);
      void loadHistory();
    });
    return () => {
      cancelled = true;
      offResync();
    };
  }, [chatId, sessionId, agentService, aiChatItemService, deviceService]);

  useEffect(() => {
    if (!chatId) {
      return;
    }
    agentService.handleSubscribe(chatId);
    const off = agentService.addEventListener(chatId, (arg: { context: RespSocketContext }) => {
      const item = arg.context.payload as AiChatItem;
      if (!item) {
        return;
      }
      acceptIncomingItem(item);
    });
    return () => {
      off();
      agentService.handleUnsubscribe(chatId);
    };
  }, [agentService, chatId]);

  return (
    <MessageBodyView
      key={historyGeneration}
      ref={viewRef}
      rows={rows}
      loading={loading}
      runDone={runDone}
      status={status}
      compaction={compaction}
      sessionId={sessionId}
      chatId={chatId}
      turnTotal={turns.length}
      summaryTick={summaryTick}
      revertItemId={revertItemId}
      lastContentKey={lastContentKey}
      copyBuffer={copyBufferRef.current}
      summaryBuffer={summaryBufferRef.current}
      turnClock={turnClockRef.current}
      onAskAnswer={onAskAnswer}
      onRevertEdit={onRevertEdit}
      onRegisterHandle={registerHandle}
    />
  );
}
