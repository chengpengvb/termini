import { useI18n } from '@/common/i18n/useI18n.ts';
import { ReactNode, Ref, useRef, useState } from 'react';
import { isNil, isNotNil } from '@/common/util/StringUtil.ts';
import { AI_TOOL_STATUS } from '@/common/const/AiToolConst.ts';
import type { AiChatItem } from '@/common/model/AiChatItem.ts';
import { Box, Button, ButtonBase, Collapse } from '@mui/material';
import { useAgentService } from '@/common/context/useAppHooks.tsx';
import { useItemHandle } from '@/page/common/hooks/useItemHandle.ts';
import { AgentCodeBlock } from '@/page/agent/common/AgentCodeBlock.tsx';
import type { ItemHandle } from '@/page/common/dto/agent/ItemHandle.ts';
import { parseDeviceBatchItems } from '@/page/common/util/toolViewUtil.ts';
import { AgentShellResult } from '@/page/agent/common/AgentShellResult.tsx';
import { KeyboardArrowDown, KeyboardArrowRight } from '@mui/icons-material';
import { RESP_EVENT_AGENT } from '@/common/model/resp/event/RespEventAgent.ts';
import { ConflictActions } from '@/page/agent/main/body/tool/common/ConflictActions.tsx';
import { DeviceBatchView } from '@/page/agent/main/body/tool/common/DeviceBatchView.tsx';
import { SuperviseNotice } from '@/page/agent/main/body/tool/common/SuperviseNotice.tsx';
import { AGENT_ANSWER_CODE, isAgentAnswerAllowed } from '@/common/model/req/event/WsAgent.ts';
import { ConflictAuditNotice } from '@/page/agent/main/body/tool/common/ConflictAuditNotice.tsx';
import { SUPERVISE_ACTION, type ShellConflictView, type ShellToolTitles, type SuperviseRound } from '@/page/common/dto/agent/ToolView.ts';

interface ShellToolViewProps {
  sessionId: string;
  codeLanguage: string;
  deviceBatch?: boolean;
  ref?: Ref<ItemHandle>;
  titles: ShellToolTitles;
  resolveConflictArgs: (args: unknown) => ShellConflictView | null | Promise<ShellConflictView | null>;
}

function readDeviceIds(args: unknown): string[] {
  if (isNil(args)) {
    return [];
  }
  const raw = (args as { device_ids?: string[] }).device_ids;
  if (!Array.isArray(raw)) {
    return [];
  }
  return raw.filter(item => item.length > 0);
}

export function ShellToolView(props: ShellToolViewProps) {
  const { t } = useI18n();
  const agentService = useAgentService();
  const [open, setOpen] = useState(false);
  const [done, setDone] = useState(false);
  const conflictPassedRef = useRef(false);
  const [detached, setDetached] = useState(false);
  const [rejected, setRejected] = useState(false);
  const [title, setTitle] = useState<string | null>(null);
  const [reqId, setReqId] = useState<string | null>(null);
  const [runId, setRunId] = useState<string | null>(null);
  const [deviceIds, setDeviceIds] = useState<string[]>([]);
  const [callId, setCallId] = useState<string | null>(null);
  const [output, setOutput] = useState<string | null>(null);
  const [decision, setDecision] = useState<number | null>(null);
  const [startedAt, setStartedAt] = useState<number | null>(null);
  const [auditReason, setAuditReason] = useState<string | null>(null);
  const [commandText, setCommandText] = useState<string | null>(null);
  const [sessionPrefix, setSessionPrefix] = useState<string | null>(null);
  const [supervise, setSupervise] = useState<SuperviseRound | null>(null);
  const [conflictArgs, setConflictArgs] = useState<ShellConflictView | null>(null);
  const { sessionId, titles, codeLanguage, deviceBatch, resolveConflictArgs, ref } = props;
  const [deviceChunks, setDeviceChunks] = useState<ReadonlyMap<string, string>>(new Map());

  function markConflictPassed(): void {
    conflictPassedRef.current = true;
  }

  function appendDeviceChunk(deviceId: string, delta: string): void {
    setDeviceChunks((prev: ReadonlyMap<string, string>) => {
      const next = new Map(prev);
      next.set(deviceId, (prev.get(deviceId) ?? '') + delta);
      return next;
    });
  }

  function resolveTitle(): string {
    if (rejected) {
      return titles.skipped;
    }
    if (detached) {
      return t('agent.tool.superviseDetach');
    }
    if (done) {
      return titles.completed;
    }
    if (isAgentAnswerAllowed(decision)) {
      return titles.running;
    }
    if (isNotNil(conflictArgs) && !conflictPassedRef.current) {
      return titles.waitingConfirm;
    }
    return titles.preparing;
  }

  function resolveTitleClassName(): string {
    if (done || rejected) {
      return 'tm-agent-tool-title';
    }
    return 'tm-agent-tool-title tm-agent-shimmer';
  }

  function applyCommandText(args: unknown): void {
    if (isNil(args)) {
      return;
    }
    const raw = args as { command?: string; commands?: unknown };
    if (isNotNil(raw.command) || Array.isArray(raw.commands)) {
      return;
    }
    void Promise.resolve(resolveConflictArgs(args)).then(resolved => {
      if (!resolved) {
        return;
      }
      setCommandText(resolved.command.replace(/\s+/g, ' ').trim());
    });
  }

  function renderHeading(): ReactNode {
    const subtitle = isNotNil(title) ? title : commandText;
    return (
      <>
        <Box component="span" className={resolveTitleClassName()}>
          {resolveTitle()}
        </Box>
        {isNotNil(subtitle) ? (
          <Box component="span" className="tm-agent-tool-subtitle" title={subtitle}>
            {subtitle}
          </Box>
        ) : null}
      </>
    );
  }

  function canDecide(): boolean {
    if (conflictPassedRef.current || done || rejected || isNil(conflictArgs)) {
      return false;
    }
    return !isNotNil(decision);
  }

  function canCancel(): boolean {
    if (done || rejected || isNil(decision)) {
      return false;
    }
    return isAgentAnswerAllowed(decision);
  }

  function handleCancel(): void {
    if (isNil(callId) || isNil(reqId) || isNil(runId)) {
      return;
    }
    agentService.handleToolCancel(sessionId, {
      data: { reqId, runId, callId },
    });
  }

  function handleToggleOpen(): void {
    setOpen((current: boolean) => !current);
  }

  function acceptSupervise(eventItem: AiChatItem): void {
    const incoming: SuperviseRound = {
      round: eventItem.context.round,
      action: eventItem.context.action,
      waitSeconds: eventItem.context.waitSeconds,
      reason: eventItem.context.reason,
    };
    setSupervise((prev: SuperviseRound | null) => (isNotNil(prev) && prev.round > incoming.round ? prev : incoming));
  }

  function isSupervising(): boolean {
    return !done && !rejected;
  }

  function onEvent(eventItem: AiChatItem): void {
    if (eventItem.type === RESP_EVENT_AGENT.ON_TOOL_CALL) {
      setReqId(eventItem.context.reqId);
      setRunId(eventItem.context.runId);
      setCallId(eventItem.context.callId);
      setStartedAt(eventItem.timestamp);
      setDeviceIds(readDeviceIds(eventItem.context.args));
      if (eventItem.context.args?.title) {
        setTitle(eventItem.context.args.title);
      }
      applyCommandText(eventItem.context.args);
      return;
    }
    if (eventItem.type === RESP_EVENT_AGENT.ON_CONFLICT) {
      if (conflictPassedRef.current || done || isNotNil(decision) || isNotNil(output)) {
        return;
      }
      setReqId(eventItem.context.reqId);
      setRunId(eventItem.context.runId);
      setCallId(eventItem.context.callId);
      setAuditReason(eventItem.context.auditReason ?? null);
      setSessionPrefix(eventItem.context.sessionPrefix ?? null);
      if (eventItem.context.args?.title) {
        setTitle(eventItem.context.args.title);
      }
      setDeviceIds((current: string[]) => (current.length > 0 ? current : readDeviceIds(eventItem.context.args)));
      void Promise.resolve(resolveConflictArgs(eventItem.context.args)).then(resolved => {
        setConflictArgs(resolved);
      });
      if (eventItem.context.autoAllowed) {
        markConflictPassed();
        setDecision(AGENT_ANSWER_CODE.ALLOW);
        setOpen(true);
      }
      return;
    }
    if (eventItem.type === RESP_EVENT_AGENT.ON_CONFLICT_ANSWERED) {
      markConflictPassed();
      setReqId(eventItem.context.reqId);
      setRunId(eventItem.context.runId);
      setCallId(eventItem.context.callId);
      setDecision(eventItem.context.code);
      if (eventItem.context.code === AGENT_ANSWER_CODE.REJECT) {
        setRejected(true);
        return;
      }
      setOpen(true);
      return;
    }
    if (eventItem.type === RESP_EVENT_AGENT.ON_TOOL_CALL_CHUNK) {
      markConflictPassed();
      if (!done) {
        setOpen(true);
      }
      const deviceId: string | undefined = eventItem.context.deviceId;
      if (deviceBatch === true && isNotNil(deviceId)) {
        appendDeviceChunk(deviceId, eventItem.context.delta);
        return;
      }
      setOutput((prev: string | null) => {
        if (isNil(prev)) {
          return eventItem.context.delta;
        }
        return prev + eventItem.context.delta;
      });
      return;
    }
    if (eventItem.type === RESP_EVENT_AGENT.ON_TOOL_SUPERVISE) {
      markConflictPassed();
      acceptSupervise(eventItem);
      return;
    }
    if (eventItem.type === RESP_EVENT_AGENT.ON_TOOL_CALL_DONE) {
      markConflictPassed();
      setDone(true);
      setOpen(false);
      setDetached(eventItem.context.status === AI_TOOL_STATUS.DETACHED);
      setOutput(eventItem.context.result);
      return;
    }
    if (eventItem.type === RESP_EVENT_AGENT.ON_DONE || eventItem.type === RESP_EVENT_AGENT.ON_CANCELLED) {
      markConflictPassed();
      if (rejected) {
        return;
      }
      setDone(true);
      setOpen(false);
    }
  }

  useItemHandle(ref, onEvent);

  if (canDecide()) {
    return (
      <Box className="tm-agent-tool-card tm-agent-tool-shell">
        <Box className="tm-agent-tool-head">
          <Box className="tm-agent-tool-heading">{renderHeading()}</Box>
          <ConflictActions sessionId={sessionId} callId={callId} sessionPrefix={sessionPrefix} />
        </Box>
        <ConflictAuditNotice reason={auditReason} />
        {isNotNil(conflictArgs) ? (
          <Box className="tm-agent-tool-command">
            <Box sx={{ maxHeight: 150, overflow: 'auto' }}>
              <AgentCodeBlock code={conflictArgs.command} language={codeLanguage} />
            </Box>
            {conflictArgs.detail ? <Box className="tm-agent-tool-cwd">{conflictArgs.detail}</Box> : null}
          </Box>
        ) : null}
      </Box>
    );
  }

  return (
    <Box className="tm-agent-tool-card tm-agent-tool-shell">
      <Box className="tm-agent-tool-head">
        <ButtonBase disableRipple className="tm-agent-tool-fold" onClick={handleToggleOpen}>
          <Box component="span" className="tm-agent-tool-chevron">
            {open ? <KeyboardArrowDown sx={{ fontSize: 16 }} /> : <KeyboardArrowRight sx={{ fontSize: 16 }} />}
          </Box>
          {renderHeading()}
        </ButtonBase>
        {canCancel() ? (
          <Button disableRipple className="tm-btn tm-btn-ghost tm-btn-compact" onClick={handleCancel}>
            {t('agent.tool.cancel')}
          </Button>
        ) : null}
      </Box>
      {isSupervising() || (isNotNil(supervise) && supervise.action !== SUPERVISE_ACTION.WAIT) ? <SuperviseNotice round={supervise} active={isSupervising()} startedAt={startedAt} /> : null}
      <Collapse in={open} unmountOnExit timeout={0}>
        <Box className="tm-agent-tool-body">
          {isNotNil(conflictArgs) ? (
            <Box className="tm-agent-tool-command">
              <Box sx={{ maxHeight: 240, overflow: 'auto' }}>
                <AgentCodeBlock code={conflictArgs.command} language={codeLanguage} />
              </Box>
              {conflictArgs.detail ? <Box className="tm-agent-tool-cwd">{conflictArgs.detail}</Box> : null}
            </Box>
          ) : null}
          {deviceBatch === true ? <DeviceBatchView done={done} deviceIds={deviceIds} chunks={deviceChunks} items={parseDeviceBatchItems(output)} /> : isNotNil(output) ? <AgentShellResult content={output} running={!done} active={open} /> : null}
        </Box>
      </Collapse>
    </Box>
  );
}
