import { useI18n } from '@/common/i18n/useI18n.ts';
import { Ref, type ReactNode, useState } from 'react';
import { isNil, isNotNil } from '@/common/util/StringUtil.ts';
import { AI_TOOL_STATUS } from '@/common/const/AiToolConst.ts';
import type { AiChatItem } from '@/common/model/AiChatItem.ts';
import { Box, Button, ButtonBase, Collapse } from '@mui/material';
import { useAgentService } from '@/common/context/useAppHooks.tsx';
import { useItemHandle } from '@/page/common/hooks/useItemHandle.ts';
import { AgentMarkdown } from '@/page/agent/common/AgentMarkdown.tsx';
import { AgentCodeLines } from '@/page/agent/common/AgentCodeLines.tsx';
import type { ItemHandle } from '@/page/common/dto/agent/ItemHandle.ts';
import { KeyboardArrowDown, KeyboardArrowRight } from '@mui/icons-material';
import type { FileReadToolTitles } from '@/page/common/dto/agent/ToolView.ts';
import { RESP_EVENT_AGENT } from '@/common/model/resp/event/RespEventAgent.ts';
import { parseNumberedLines, resolveLanguageByPath } from '@/page/common/util/codeHighlight.ts';
import { resolveToolBatchEntries, splitToolBatchResult, type ToolBatchEntry } from '@/page/common/util/toolBatchEntry.ts';

interface FileReadToolViewProps {
  markdown?: boolean;
  ref?: Ref<ItemHandle>;
  cancelSessionId?: string;
  titles: FileReadToolTitles;
  onOpenPath?: (path: string) => void;
  pending?: ReactNode;
}

interface ToolCallRef {
  reqId: string;
  runId: string;
  callId: string;
}

export function FileReadToolView(props: FileReadToolViewProps) {
  const { t } = useI18n();
  const agentService = useAgentService();
  const [status, setStatus] = useState<number>(AI_TOOL_STATUS.RUNNING);
  const [stream, setStream] = useState<string | null>(null);
  const [callRef, setCallRef] = useState<ToolCallRef | null>(null);
  const { ref, titles, markdown, cancelSessionId, onOpenPath, pending } = props;

  function renderBody(text: string, path: string | null) {
    if (markdown === true) {
      return <AgentMarkdown content={text} className="tm-agent-md tm-agent-tool-markdown" />;
    }
    const lines = parseNumberedLines(text);
    if (isNotNil(lines)) {
      return <AgentCodeLines lines={lines} language={resolveLanguageByPath(path)} />;
    }
    return (
      <Box component="pre" className="tm-agent-shell-output-static">
        {text}
      </Box>
    );
  }
  const [result, setResult] = useState<string | null>(null);
  const [entries, setEntries] = useState<ToolBatchEntry[]>([]);
  const [openKeys, setOpenKeys] = useState<ReadonlySet<string>>(new Set());

  function resolveTitle(): string {
    if (status === AI_TOOL_STATUS.DETACHED) {
      return titles.detached ?? titles.reading;
    }
    if (status === AI_TOOL_STATUS.FAILED) {
      return titles.failed ?? titles.done;
    }
    if (status === AI_TOOL_STATUS.CANCELLED || status === AI_TOOL_STATUS.KILLED) {
      return titles.cancelled ?? titles.done;
    }
    return status === AI_TOOL_STATUS.DONE ? titles.done : titles.reading;
  }

  function resolveTitleClassName(): string {
    if (isTerminal()) {
      return 'tm-agent-tool-title';
    }
    return 'tm-agent-tool-title tm-agent-shimmer';
  }

  function handleToggle(key: string): void {
    setOpenKeys((current: ReadonlySet<string>) => {
      const next = new Set(current);
      if (next.has(key)) {
        next.delete(key);
      } else {
        next.add(key);
      }
      return next;
    });
  }

  function canCancel(): boolean {
    return isNotNil(cancelSessionId) && isNotNil(callRef) && !isTerminal();
  }

  function isTerminal(): boolean {
    return status === AI_TOOL_STATUS.DONE || status === AI_TOOL_STATUS.FAILED || status === AI_TOOL_STATUS.CANCELLED || status === AI_TOOL_STATUS.KILLED;
  }

  function handleCancel(): void {
    if (isNil(cancelSessionId) || isNil(callRef)) {
      return;
    }
    agentService.handleToolCancel(cancelSessionId, { data: { reqId: callRef.reqId, runId: callRef.runId, callId: callRef.callId } });
  }

  function onEvent(eventItem: AiChatItem): void {
    if (eventItem.type === RESP_EVENT_AGENT.ON_TOOL_CALL) {
      setEntries(resolveToolBatchEntries(eventItem.context.args));
      setCallRef({ reqId: eventItem.context.reqId, runId: eventItem.context.runId, callId: eventItem.context.callId });
      return;
    }
    if (eventItem.type === RESP_EVENT_AGENT.ON_TOOL_CALL_CHUNK) {
      setStatus(AI_TOOL_STATUS.RUNNING);
      setStream((prev: string | null) => (isNil(prev) ? eventItem.context.delta : prev + eventItem.context.delta));
      return;
    }
    if (eventItem.type === RESP_EVENT_AGENT.ON_TOOL_CALL_DONE) {
      setStatus(eventItem.context.status ?? AI_TOOL_STATUS.DONE);
      setResult(eventItem.context.result);
      return;
    }
    if (eventItem.type === RESP_EVENT_AGENT.ON_DONE || eventItem.type === RESP_EVENT_AGENT.ON_CANCELLED) {
      setStatus(eventItem.type === RESP_EVENT_AGENT.ON_CANCELLED ? AI_TOOL_STATUS.CANCELLED : AI_TOOL_STATUS.DONE);
    }
  }

  useItemHandle(ref, onEvent);

  const shown = isTerminal() ? result : (stream ?? result);
  const sections = splitToolBatchResult(shown);

  function renderCancel() {
    if (!canCancel()) {
      return null;
    }
    return (
      <Button disableRipple className="tm-btn tm-btn-ghost tm-btn-compact" onClick={handleCancel}>
        {t('agent.tool.cancel')}
      </Button>
    );
  }

  function resolveBody(entry: ToolBatchEntry): string | null {
    const section = sections.get(entry.label);
    if (isNotNil(section) && section.length > 0) {
      return section;
    }
    return entries.length <= 1 ? shown : null;
  }

  function renderLabel(entry: ToolBatchEntry) {
    const canOpen = isNotNil(onOpenPath) && isNotNil(entry.openPath);
    return (
      <Box
        component="span"
        title={entry.openPath ?? entry.label}
        className={canOpen ? 'tm-agent-tool-path is-link' : 'tm-agent-tool-path'}
        onClick={
          canOpen
            ? event => {
                event.preventDefault();
                event.stopPropagation();
                onOpenPath(entry.openPath as string);
              }
            : undefined
        }
      >
        {entry.label}
      </Box>
    );
  }

  function renderRow(entry: ToolBatchEntry) {
    const open = openKeys.has(entry.key);
    const body = resolveBody(entry);
    return (
      <Box key={entry.key} className="tm-agent-tool-card">
        <Box className="tm-agent-tool-head">
          <ButtonBase disableRipple className="tm-agent-tool-fold" onClick={() => handleToggle(entry.key)}>
            <Box component="span" className="tm-agent-tool-chevron">
              {open ? <KeyboardArrowDown sx={{ fontSize: 16 }} /> : <KeyboardArrowRight sx={{ fontSize: 16 }} />}
            </Box>
            <Box component="span" className={resolveTitleClassName()}>
              {resolveTitle()}
            </Box>
            {renderLabel(entry)}
          </ButtonBase>
          {entries.length <= 1 ? renderCancel() : null}
        </Box>
        <Collapse in={open} unmountOnExit timeout={0}>
          <Box className="tm-agent-tool-body">{isNotNil(body) ? renderBody(body, entry.openPath ?? entry.label) : null}</Box>
        </Collapse>
      </Box>
    );
  }

  function renderTool(): ReactNode {
  if (entries.length === 0) {
    const open = openKeys.has('single');
    return (
      <Box className="tm-agent-tool-card">
        <Box className="tm-agent-tool-head">
          <ButtonBase disableRipple className="tm-agent-tool-fold" onClick={() => handleToggle('single')}>
            <Box component="span" className="tm-agent-tool-chevron">
              {open ? <KeyboardArrowDown sx={{ fontSize: 16 }} /> : <KeyboardArrowRight sx={{ fontSize: 16 }} />}
            </Box>
            <Box component="span" className={resolveTitleClassName()}>
              {resolveTitle()}
            </Box>
          </ButtonBase>
          {renderCancel()}
        </Box>
        <Collapse in={open} unmountOnExit timeout={0}>
          <Box className="tm-agent-tool-body">{isNil(shown) ? null : renderBody(shown, null)}</Box>
        </Collapse>
      </Box>
    );
  }

  if (entries.length === 1) {
    return renderRow(entries[0]);
  }

  const groupOpen = openKeys.has('batch');
  const groupTitle = isTerminal() ? (titles.batchDone?.(entries.length) ?? resolveTitle()) : (titles.batchReading?.(entries.length) ?? resolveTitle());
  return (
    <Box className="tm-agent-tool-card tm-agent-tool-batch-group">
      <Box className="tm-agent-tool-head">
        <ButtonBase disableRipple className="tm-agent-tool-fold" onClick={() => handleToggle('batch')}>
          <Box component="span" className="tm-agent-tool-chevron">
            {groupOpen ? <KeyboardArrowDown sx={{ fontSize: 16 }} /> : <KeyboardArrowRight sx={{ fontSize: 16 }} />}
          </Box>
          <Box component="span" className={resolveTitleClassName()} title={entries.map(entry => entry.openPath ?? entry.label).join('\n')}>
            {groupTitle}
          </Box>
        </ButtonBase>
        {renderCancel()}
      </Box>
      <Collapse in={groupOpen} unmountOnExit timeout={0}>
        <Box className="tm-agent-tool-body">
          <Box className="tm-agent-tool-batch">{entries.map(entry => renderRow(entry))}</Box>
        </Box>
      </Collapse>
    </Box>
  );
  }

  return <>{renderTool()}{!isTerminal() ? pending : null}</>;
}
