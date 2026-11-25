import { Ref, useRef, useState } from 'react';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { Box, ButtonBase, Collapse } from '@mui/material';
import { isNil, isNotNil } from '@/common/util/StringUtil.ts';
import type { AiChatItem } from '@/common/model/AiChatItem.ts';
import { useItemHandle } from '@/page/common/hooks/useItemHandle.ts';
import { AGENT_ANSWER_CODE } from '@/common/model/req/event/WsAgent.ts';
import { AgentCodeBlock } from '@/page/agent/common/AgentCodeBlock.tsx';
import type { ItemHandle } from '@/page/common/dto/agent/ItemHandle.ts';
import { AI_TOOL_ID, resolveAiToolId } from '@/common/const/AiToolConst.ts';
import { KeyboardArrowDown, KeyboardArrowRight } from '@mui/icons-material';
import { RESP_EVENT_AGENT } from '@/common/model/resp/event/RespEventAgent.ts';
import { ConflictActions } from '@/page/agent/main/body/tool/common/ConflictActions.tsx';

interface ItemTextProps {
  sessionId: string;
  ref?: Ref<ItemHandle>;
}

const TITLE_DONE_KEYS: Record<number, string> = {
  [AI_TOOL_ID.DEVICE_LIST]: 'agent.tool.deviceList',
  [AI_TOOL_ID.GROUP_LIST]: 'agent.tool.groupList',
  [AI_TOOL_ID.SKILL_LIST]: 'agent.tool.skillList',
  [AI_TOOL_ID.CHAT_HISTORY]: 'agent.tool.chatHistory',
  [AI_TOOL_ID.JOB_INDICATOR_DRAFT]: 'agent.tool.jobIndicatorDraft',
  [AI_TOOL_ID.JOB_INDICATOR_LIST]: 'agent.tool.jobIndicatorList',
  [AI_TOOL_ID.SNMP_PROBE]: 'agent.tool.snmpProbe',
  [AI_TOOL_ID.DESKTOP_SNAPSHOT]: 'agent.tool.desktopSnapshot',
  [AI_TOOL_ID.TASK_WAIT]: 'agent.tool.taskWaitDone',
  [AI_TOOL_ID.TASK_LOG]: 'agent.tool.taskLogDone',
  [AI_TOOL_ID.TASK_LIST]: 'agent.tool.taskListDone',
  [AI_TOOL_ID.TASK_KILL]: 'agent.tool.taskKillDone',
  [AI_TOOL_ID.LOOP_START]: 'agent.tool.loopStartDone',
  [AI_TOOL_ID.LOOP_STOP]: 'agent.tool.loopStopDone',
  [AI_TOOL_ID.DERIVED_LIST]: 'agent.tool.derivedListDone',
  [AI_TOOL_ID.TOOL_SEARCH]: 'agent.tool.toolSearchCompleted',
  [AI_TOOL_ID.ENTITY_LIST]: 'agent.tool.entityList',
  [AI_TOOL_ID.ENTITY_DESCRIBE]: 'agent.tool.entityDescribe',
  [AI_TOOL_ID.ENTITY_CALL]: 'agent.tool.entityCall',
  [AI_TOOL_ID.DEVICE_IMPORT]: 'agent.tool.deviceImport',
};

const TITLE_PENDING_KEYS: Record<number, string> = {
  [AI_TOOL_ID.DEVICE_LIST]: 'agent.tool.queryingDevice',
  [AI_TOOL_ID.GROUP_LIST]: 'agent.tool.queryingGroup',
  [AI_TOOL_ID.SKILL_LIST]: 'agent.tool.queryingSkill',
  [AI_TOOL_ID.CHAT_HISTORY]: 'agent.tool.queryingHistory',
  [AI_TOOL_ID.JOB_INDICATOR_DRAFT]: 'agent.tool.jobIndicatorDrafting',
  [AI_TOOL_ID.JOB_INDICATOR_LIST]: 'agent.tool.queryingJobIndicator',
  [AI_TOOL_ID.SNMP_PROBE]: 'agent.tool.snmpProbing',
  [AI_TOOL_ID.DESKTOP_SNAPSHOT]: 'agent.tool.queryingDesktop',
  [AI_TOOL_ID.TASK_WAIT]: 'agent.tool.taskWaitRunning',
  [AI_TOOL_ID.TASK_LOG]: 'agent.tool.taskLogRunning',
  [AI_TOOL_ID.TASK_LIST]: 'agent.tool.taskListRunning',
  [AI_TOOL_ID.TASK_KILL]: 'agent.tool.taskKillRunning',
  [AI_TOOL_ID.LOOP_START]: 'agent.tool.loopStartRunning',
  [AI_TOOL_ID.LOOP_STOP]: 'agent.tool.loopStopRunning',
  [AI_TOOL_ID.DERIVED_LIST]: 'agent.tool.derivedListRunning',
  [AI_TOOL_ID.TOOL_SEARCH]: 'agent.tool.toolSearchRunning',
  [AI_TOOL_ID.ENTITY_LIST]: 'agent.tool.queryingEntityList',
  [AI_TOOL_ID.ENTITY_DESCRIBE]: 'agent.tool.queryingEntityDescribe',
  [AI_TOOL_ID.ENTITY_CALL]: 'agent.tool.queryingEntityCall',
  [AI_TOOL_ID.DEVICE_IMPORT]: 'agent.tool.queryingDeviceImport',
};

export function ItemText(props: ItemTextProps) {
  const { t } = useI18n();
  const { ref, sessionId } = props;
  const [open, setOpen] = useState(false);
  const [done, setDone] = useState(false);
  const [callId, setCallId] = useState<string | null>(null);
  const [decision, setDecision] = useState<number | null>(null);
  const [toolId, setToolId] = useState<number | null>(null);
  const [result, setResult] = useState<string | null>(null);
  const conflictPassedRef = useRef(false);

  function resolveTitle(): string {
    if (isNil(toolId)) {
      if (done) {
        return t('agent.tool.queryResult');
      }
      return t('agent.tool.querying');
    }
    const titleMap = done ? TITLE_DONE_KEYS : TITLE_PENDING_KEYS;
    const titleKey = titleMap[toolId];
    if (!titleKey) {
      if (done) {
        return t('agent.tool.queryResult');
      }
      return t('agent.tool.querying');
    }
    return t(titleKey);
  }

  function resolveTitleClassName(): string {
    if (done) {
      return 'tm-agent-fold-label';
    }
    return 'tm-agent-fold-label tm-agent-shimmer';
  }

  function parseJson(raw: string): unknown | null {
    try {
      return JSON.parse(raw);
    } catch {
      return null;
    }
  }

  function resolveLanguage(raw: string): string {
    if (parseJson(raw) != null) {
      return 'json';
    }
    return 'text';
  }

  function resolveDisplayCode(raw: string): string {
    const parsed = parseJson(raw);
    if (parsed == null) {
      return raw;
    }
    return JSON.stringify(parsed, null, 2);
  }

  function handleToggleOpen(): void {
    setOpen((current: boolean) => !current);
  }

  function markConflictPassed(): void {
    conflictPassedRef.current = true;
  }

  function canDecide(): boolean {
    if (conflictPassedRef.current || done || isNil(callId)) {
      return false;
    }
    return isNil(decision);
  }

    function resolveConflictArgs(args: unknown): string | null {
        if (isNil(args)) {
            return null;
        }
        return JSON.stringify(args, null, 2);
    }

  function onEvent(eventItem: AiChatItem): void {
    if (eventItem.type === RESP_EVENT_AGENT.ON_TOOL_CALL) {
      const nextToolId = resolveAiToolId(eventItem.context.toolId);
      if (isNotNil(nextToolId)) {
        setToolId(nextToolId);
      }
      return;
    }
    if (eventItem.type === RESP_EVENT_AGENT.ON_CONFLICT) {
      if (conflictPassedRef.current || done || isNotNil(decision)) {
        return;
      }
      setCallId(eventItem.context.callId);
      const argsText = resolveConflictArgs(eventItem.context.args);
      if (isNotNil(argsText)) {
        setResult(argsText);
      }
      if (eventItem.context.autoAllowed) {
        markConflictPassed();
        setDecision(AGENT_ANSWER_CODE.ALLOW);
        return;
      }
      setOpen(true);
      return;
    }
    if (eventItem.type === RESP_EVENT_AGENT.ON_CONFLICT_ANSWERED) {
      markConflictPassed();
      setCallId(eventItem.context.callId);
      setDecision(eventItem.context.code);
      if (eventItem.context.code === AGENT_ANSWER_CODE.REJECT) {
        setDone(true);
      }
      return;
    }
    if (eventItem.type === RESP_EVENT_AGENT.ON_TOOL_CALL_CHUNK) {
      setDone(false);
      setResult((prev: string | null) => {
        if (isNil(prev)) {
          return eventItem.context.delta;
        }
        return prev + eventItem.context.delta;
      });
      return;
    }
    if (eventItem.type === RESP_EVENT_AGENT.ON_TOOL_CALL_DONE) {
      markConflictPassed();
      setDone(true);
      setResult(eventItem.context.result);
      return;
    }
    if (eventItem.type === RESP_EVENT_AGENT.ON_DONE || eventItem.type === RESP_EVENT_AGENT.ON_CANCELLED) {
      markConflictPassed();
      setDone(true);
    }
  }

  useItemHandle(ref, onEvent);

  return (
    <Box className="tm-agent-fold-row">
      <ButtonBase disableRipple className="tm-agent-fold-toggle" onClick={handleToggleOpen}>
        <Box component="span" className={resolveTitleClassName()}>
          {resolveTitle()}
        </Box>
        <Box component="span" className="tm-agent-fold-chevron">
          {open ? <KeyboardArrowDown sx={{ fontSize: 16 }} /> : <KeyboardArrowRight sx={{ fontSize: 16 }} />}
        </Box>
      </ButtonBase>
      {canDecide() ? <ConflictActions sessionId={sessionId} callId={callId} /> : null}
      <Collapse in={open} unmountOnExit timeout={0}>
        <Box className="tm-agent-fold-body">{isNotNil(result) ? <AgentCodeBlock code={resolveDisplayCode(result)} language={resolveLanguage(result)} /> : null}</Box>
      </Collapse>
    </Box>
  );
}
