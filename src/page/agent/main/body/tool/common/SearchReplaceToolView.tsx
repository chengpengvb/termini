import { useI18n } from '@/common/i18n/useI18n.ts';
import { Box, ButtonBase, Collapse } from '@mui/material';
import { isNil, isNotNil } from '@/common/util/StringUtil.ts';
import { Ref, useRef, useState, type ReactNode } from 'react';
import type { AiChatItem } from '@/common/model/AiChatItem.ts';
import { useItemHandle } from '@/page/common/hooks/useItemHandle.ts';
import { AgentCodeBlock } from '@/page/agent/common/AgentCodeBlock.tsx';
import { groupReplaceByFile } from '@/page/common/util/toolViewUtil.ts';
import type { ItemHandle } from '@/page/common/dto/agent/ItemHandle.ts';
import { KeyboardArrowDown, KeyboardArrowRight } from '@mui/icons-material';
import { RESP_EVENT_AGENT } from '@/common/model/resp/event/RespEventAgent.ts';
import { ConflictActions } from '@/page/agent/main/body/tool/common/ConflictActions.tsx';
import { ReplaceDiffView } from '@/page/agent/main/body/tool/common/ReplaceDiffView.tsx';
import { AGENT_ANSWER_CODE, isAgentAnswerAllowed } from '@/common/model/req/event/WsAgent.ts';
import { ConflictAuditNotice } from '@/page/agent/main/body/tool/common/ConflictAuditNotice.tsx';
import type { ReplaceConflictView, SearchReplaceToolTitles } from '@/page/common/dto/agent/ToolView.ts';

interface SearchReplaceToolViewProps {
  sessionId: string;
  ref?: Ref<ItemHandle>;
  titles: SearchReplaceToolTitles;
  onOpenPath?: (path: string) => void;
  resolveCompletedTitle: (displayName: string) => string;
  resolveConflictArgs: (args: unknown) => ReplaceConflictView[];
}

export function SearchReplaceToolView(props: SearchReplaceToolViewProps) {
  const { tf } = useI18n();
  const [open, setOpen] = useState(false);
  const [done, setDone] = useState(false);
  const conflictPassedRef = useRef(false);
  const [rejected, setRejected] = useState(false);
  const [path, setPath] = useState<string | null>(null);
  const [callId, setCallId] = useState<string | null>(null);
  const [result, setResult] = useState<string | null>(null);
  const [decision, setDecision] = useState<number | null>(null);
  const [auditReason, setAuditReason] = useState<string | null>(null);
  const [openRows, setOpenRows] = useState<ReadonlySet<string>>(new Set());
  const [conflictArgs, setConflictArgs] = useState<ReplaceConflictView[]>([]);
  const { sessionId, titles, resolveCompletedTitle, resolveConflictArgs, ref, onOpenPath } = props;
  const files = groupReplaceByFile(conflictArgs);

  function markConflictPassed(): void {
    conflictPassedRef.current = true;
  }

  function resolveTitle(): string {
    if (rejected) {
      return titles.skipped;
    }
    if (done) {
      // 单文件时标题里带上文件名; 多文件各行自己显示路径, 标题保持统一措辞
      if (files.length === 1) {
        return resolveCompletedTitle(files[0].displayName);
      }
      return titles.completedFallback;
    }
    if (isAgentAnswerAllowed(decision)) {
      return titles.running;
    }
    if (conflictArgs.length > 0 && !conflictPassedRef.current) {
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

  function canDecide(): boolean {
    if (conflictPassedRef.current || done || rejected || conflictArgs.length === 0) {
      return false;
    }
    return !isNotNil(decision);
  }

  function handleToggleOpen(): void {
    setOpen((current: boolean) => !current);
  }

  function handleToggleRow(key: string): void {
    setOpenRows((current: ReadonlySet<string>) => {
      const next = new Set(current);
      if (next.has(key)) {
        next.delete(key);
      } else {
        next.add(key);
      }
      return next;
    });
  }

  function renderOnePath(label: string, openPath: string | null): ReactNode {
    const canOpen = isNotNil(onOpenPath) && isNotNil(openPath);
    return (
      <Box
        key={label}
        component="span"
        title={openPath ?? label}
        className={canOpen ? 'tm-agent-tool-path is-link' : 'tm-agent-tool-path'}
        onClick={
          canOpen
            ? event => {
                event.preventDefault();
                event.stopPropagation();
                onOpenPath(openPath as string);
              }
            : undefined
        }
      >
        {label}
      </Box>
    );
  }

  function renderPath(): ReactNode {
    if (files.length === 1) {
      return renderOnePath(files[0].path, files[0].fullPath);
    }
    if (isNil(path)) {
      return null;
    }
    return renderOnePath(path, path);
  }

  function renderCount(): ReactNode {
    if (files.length !== 1 || conflictArgs.length <= 1) {
      return null;
    }
    return (
      <Box component="span" className="tm-agent-tool-path">
        {tf('agent.tool.editCount', { n: conflictArgs.length })}
      </Box>
    );
  }

  function renderDiff(args: ReplaceConflictView, index: number): ReactNode {
    return <ReplaceDiffView key={`${args.path}#${index}`} args={args} />;
  }

  function onEvent(eventItem: AiChatItem): void {
    if (eventItem.type === RESP_EVENT_AGENT.ON_TOOL_CALL) {
      setCallId(eventItem.context.callId);
      if (eventItem.context.args?.path) {
        setPath(eventItem.context.args.path);
      }
      // 调用阶段就把批量的文件清单摆出来, 不用等确认事件
      const called = resolveConflictArgs(eventItem.context.args);
      if (called.length > 0) {
        setConflictArgs(called);
      }
      return;
    }
    if (eventItem.type === RESP_EVENT_AGENT.ON_CONFLICT) {
      if (conflictPassedRef.current || done || isNotNil(decision)) {
        return;
      }
      setCallId(eventItem.context.callId);
      setAuditReason(eventItem.context.auditReason ?? null);
      setConflictArgs(resolveConflictArgs(eventItem.context.args));
      //自动审核已放行，本事件只是把改动带过来留痕，直接按已同意渲染，避免闪出确认按钮
      if (eventItem.context.autoAllowed) {
        markConflictPassed();
        setDecision(AGENT_ANSWER_CODE.ALLOW);
        setOpen(true);
      }
      return;
    }
    if (eventItem.type === RESP_EVENT_AGENT.ON_CONFLICT_ANSWERED) {
      markConflictPassed();
      setCallId(eventItem.context.callId);
      setDecision(eventItem.context.code);
      if (eventItem.context.code === AGENT_ANSWER_CODE.REJECT) {
        setRejected(true);
        return;
      }
      setOpen(true);
      return;
    }
    if (eventItem.type === RESP_EVENT_AGENT.ON_TOOL_CALL_DONE) {
      markConflictPassed();
      setDone(true);
      setOpen(false);
      setResult(eventItem.context.result);
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

  // 待确认: 整批只问一次, 但每个文件的改动各自成块展示
  if (canDecide()) {
    return (
      <Box className="tm-agent-tool-card">
        <Box className="tm-agent-tool-head">
          <Box className={resolveTitleClassName()}>{resolveTitle()}</Box>
          {renderPath()}
          {renderCount()}
          <ConflictActions sessionId={sessionId} callId={callId} />
        </Box>
        <ConflictAuditNotice reason={auditReason} />
        {conflictArgs.map((item, index) => renderDiff(item, index))}
      </Box>
    );
  }

  // 一次改多个文件时: 外层一行汇总, 展开后每个文件一行, 同一文件的多处改动收在同一行里
  if (files.length > 1) {
    return (
      <Box className="tm-agent-tool-card tm-agent-tool-batch-group">
        <Box className="tm-agent-tool-head">
          <ButtonBase disableRipple className="tm-agent-tool-fold" onClick={handleToggleOpen}>
            <Box component="span" className="tm-agent-tool-chevron">
              {open ? <KeyboardArrowDown sx={{ fontSize: 16 }} /> : <KeyboardArrowRight sx={{ fontSize: 16 }} />}
            </Box>
            <Box component="span" className={resolveTitleClassName()} title={files.map(file => file.fullPath ?? file.path).join('\n')}>
              {resolveTitle()}
            </Box>
            <Box component="span" className="tm-agent-tool-path">
              {tf('agent.tool.fileCount', { n: files.length })}
            </Box>
          </ButtonBase>
        </Box>
        <Collapse in={open} unmountOnExit timeout={0}>
          <Box className="tm-agent-tool-body">
            <Box className="tm-agent-tool-batch">
              {files.map(file => {
                const rowOpen = openRows.has(file.key);
                return (
                  <Box key={file.key} className="tm-agent-tool-card">
                    <Box className="tm-agent-tool-head">
                      <ButtonBase disableRipple className="tm-agent-tool-fold" onClick={() => handleToggleRow(file.key)}>
                        <Box component="span" className="tm-agent-tool-chevron">
                          {rowOpen ? <KeyboardArrowDown sx={{ fontSize: 16 }} /> : <KeyboardArrowRight sx={{ fontSize: 16 }} />}
                        </Box>
                        <Box component="span" className="tm-agent-tool-title">
                          {file.displayName}
                        </Box>
                        {renderOnePath(file.path, file.fullPath)}
                        {file.edits.length > 1 ? (
                          <Box component="span" className="tm-agent-tool-path">
                            {tf('agent.tool.editCount', { n: file.edits.length })}
                          </Box>
                        ) : null}
                      </ButtonBase>
                    </Box>
                    <Collapse in={rowOpen} unmountOnExit timeout={0}>
                      <Box className="tm-agent-tool-body">{file.edits.map((item, index) => renderDiff(item, index))}</Box>
                    </Collapse>
                  </Box>
                );
              })}
            </Box>
          </Box>
        </Collapse>
      </Box>
    );
  }

  return (
    <Box className="tm-agent-tool-card">
      <Box className="tm-agent-tool-head">
        <ButtonBase disableRipple className="tm-agent-tool-fold" onClick={handleToggleOpen}>
          <Box component="span" className="tm-agent-tool-chevron">
            {open ? <KeyboardArrowDown sx={{ fontSize: 16 }} /> : <KeyboardArrowRight sx={{ fontSize: 16 }} />}
          </Box>
          <Box component="span" className={resolveTitleClassName()}>
            {resolveTitle()}
          </Box>
          {renderPath()}
          {renderCount()}
        </ButtonBase>
      </Box>
      <Collapse in={open} unmountOnExit timeout={0}>
        <Box className="tm-agent-tool-body">
          {conflictArgs.map((item, index) => renderDiff(item, index))}
          {conflictArgs.length === 0 && isNotNil(result) ? <AgentCodeBlock code={result} language="text" /> : null}
        </Box>
      </Collapse>
    </Box>
  );
}
