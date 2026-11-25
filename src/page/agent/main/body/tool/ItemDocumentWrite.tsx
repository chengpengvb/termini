import { useI18n } from '@/common/i18n/useI18n.ts';
import { Box, ButtonBase, Collapse } from '@mui/material';
import { isNil, isNotNil } from '@/common/util/StringUtil.ts';
import { Ref, useRef, useState, type MouseEvent } from 'react';
import type { AiChatItem } from '@/common/model/AiChatItem.ts';
import { useItemHandle } from '@/page/common/hooks/useItemHandle.ts';
import { resolveErrorText } from '@/common/model/resp/ResultModel.ts';
import { AgentCodeBlock } from '@/page/agent/common/AgentCodeBlock.tsx';
import { useSystemUtilService } from '@/common/context/useAppHooks.tsx';
import type { ItemHandle } from '@/page/common/dto/agent/ItemHandle.ts';
import { AI_TOOL_ID, resolveAiToolId } from '@/common/const/AiToolConst.ts';
import { RESP_EVENT_AGENT } from '@/common/model/resp/event/RespEventAgent.ts';
import { ConflictActions } from '@/page/agent/main/body/tool/common/ConflictActions.tsx';
import { AGENT_ANSWER_CODE, isAgentAnswerAllowed } from '@/common/model/req/event/WsAgent.ts';
import { ConflictAuditNotice } from '@/page/agent/main/body/tool/common/ConflictAuditNotice.tsx';
import { FolderOpen, KeyboardArrowDown, KeyboardArrowRight, OpenInNew } from '@mui/icons-material';

interface ItemDocumentWriteProps {
  sessionId: string;
  ref?: Ref<ItemHandle>;
}

interface DocumentWriteArgs {
  path: string;
  source: string | null;
}

interface TitleKeys {
  done: string;
  running: string;
  waiting: string;
  skipped: string;
}

const TITLE_KEYS: Record<number, TitleKeys> = {
  [AI_TOOL_ID.DOCX_WRITE]: { done: 'agent.tool.docxWriteDone', running: 'agent.tool.docxWriteRunning', waiting: 'agent.tool.docxWriteWaiting', skipped: 'agent.tool.docxWriteSkipped' },
  [AI_TOOL_ID.PDF_WRITE]: { done: 'agent.tool.pdfWriteDone', running: 'agent.tool.pdfWriteRunning', waiting: 'agent.tool.pdfWriteWaiting', skipped: 'agent.tool.pdfWriteSkipped' },
  [AI_TOOL_ID.XLSX_WRITE]: { done: 'agent.tool.xlsxWriteDone', running: 'agent.tool.xlsxWriteRunning', waiting: 'agent.tool.xlsxWriteWaiting', skipped: 'agent.tool.xlsxWriteSkipped' },
};

export function ItemDocumentWrite(props: ItemDocumentWriteProps) {
  const { sessionId, ref } = props;
  const { t, tf } = useI18n();
  const [open, setOpen] = useState(false);
  const [done, setDone] = useState(false);
  const conflictPassedRef = useRef(false);
  const systemUtilService = useSystemUtilService();
  const [rejected, setRejected] = useState(false);
  const [toolId, setToolId] = useState<number | null>(null);
  const [callId, setCallId] = useState<string | null>(null);
  const [result, setResult] = useState<string | null>(null);
  const [decision, setDecision] = useState<number | null>(null);
  const [openError, setOpenError] = useState<string | null>(null);
  const [auditReason, setAuditReason] = useState<string | null>(null);
  const [args, setArgs] = useState<DocumentWriteArgs | null>(null);
  // 只有真正写成功的文件才给"打开": 被跳过或还在生成时路径上可能没有文件
  const canOpenFile = done && !rejected && isNotNil(args) && isNil(failedResult());

  function failedResult(): string | null {
    return isNotNil(result) && result.startsWith('<error>') ? result : null;
  }

  function handleOpenFile(event: MouseEvent): void {
    event.stopPropagation();
    if (isNil(args)) {
      return;
    }
    systemUtilService.openLocalPath(args.path).catch((error: unknown) => {
      setOpenError(tf('agent.tool.documentOpenFailed', { error: resolveErrorText(error) }));
    });
  }

  function handleRevealFile(event: MouseEvent): void {
    event.stopPropagation();
    if (isNil(args)) {
      return;
    }
    systemUtilService.revealLocalPath(args.path).catch((error: unknown) => {
      setOpenError(tf('agent.tool.documentOpenFailed', { error: resolveErrorText(error) }));
    });
  }

  function titleKeys(): TitleKeys {
    return (isNotNil(toolId) ? TITLE_KEYS[toolId] : undefined) ?? TITLE_KEYS[AI_TOOL_ID.DOCX_WRITE];
  }

  function resolveTitle(): string {
    const keys = titleKeys();
    if (rejected) {
      return t(keys.skipped);
    }
    if (done) {
      return t(keys.done);
    }
    if (isAgentAnswerAllowed(decision)) {
      return t(keys.running);
    }
    if (isNotNil(args) && !conflictPassedRef.current) {
      return t(keys.waiting);
    }
    return t(keys.running);
  }

  function resolveTitleClassName(): string {
    if (done || rejected) {
      return 'tm-agent-tool-title';
    }
    return 'tm-agent-tool-title tm-agent-shimmer';
  }

  function canDecide(): boolean {
    if (conflictPassedRef.current || done || rejected || isNil(args)) {
      return false;
    }
    return isNil(decision);
  }

  function resolveArgs(raw: unknown): DocumentWriteArgs | null {
    if (isNil(raw)) {
      return null;
    }
    const record = raw as DocumentWriteArgs;
    if (isNil(record.path)) {
      return null;
    }
    return { path: record.path, source: record.source ?? null };
  }

  function renderArgs() {
    if (isNil(args)) {
      return null;
    }
    const lines = [args.path];
    if (isNotNil(args.source)) {
      lines.push(`${t('agent.tool.documentWriteSource')}: ${args.source}`);
    }
    return (
      <Box className="tm-agent-tool-command">
        <AgentCodeBlock code={lines.join('\n')} language="text" />
      </Box>
    );
  }

  function onEvent(eventItem: AiChatItem): void {
    if (eventItem.type === RESP_EVENT_AGENT.ON_TOOL_CALL) {
      setCallId(eventItem.context.callId);
      const nextToolId = resolveAiToolId(eventItem.context.toolId);
      if (isNotNil(nextToolId)) {
        setToolId(nextToolId);
      }
      const parsed = resolveArgs(eventItem.context.args);
      if (isNotNil(parsed)) {
        setArgs(parsed);
      }
      return;
    }
    if (eventItem.type === RESP_EVENT_AGENT.ON_CONFLICT) {
      if (conflictPassedRef.current || done || isNotNil(decision)) {
        return;
      }
      setCallId(eventItem.context.callId);
      setAuditReason(eventItem.context.auditReason ?? null);
      const parsed = resolveArgs(eventItem.context.args);
      if (isNotNil(parsed)) {
        setArgs(parsed);
      }
      if (eventItem.context.autoAllowed) {
        conflictPassedRef.current = true;
        setDecision(AGENT_ANSWER_CODE.ALLOW);
      }
      return;
    }
    if (eventItem.type === RESP_EVENT_AGENT.ON_CONFLICT_ANSWERED) {
      conflictPassedRef.current = true;
      setCallId(eventItem.context.callId);
      setDecision(eventItem.context.code);
      if (eventItem.context.code === AGENT_ANSWER_CODE.REJECT) {
        setRejected(true);
      }
      return;
    }
    if (eventItem.type === RESP_EVENT_AGENT.ON_TOOL_CALL_DONE) {
      conflictPassedRef.current = true;
      setDone(true);
      setResult(eventItem.context.result);
      return;
    }
    if (eventItem.type === RESP_EVENT_AGENT.ON_DONE || eventItem.type === RESP_EVENT_AGENT.ON_CANCELLED) {
      conflictPassedRef.current = true;
      if (!rejected) {
        setDone(true);
      }
    }
  }

  useItemHandle(ref, onEvent);

  if (canDecide()) {
    return (
      <Box className="tm-agent-tool-card">
        <Box className="tm-agent-tool-head">
          <Box className={resolveTitleClassName()}>{resolveTitle()}</Box>
          <ConflictActions sessionId={sessionId} callId={callId} />
        </Box>
        <ConflictAuditNotice reason={auditReason} />
        {renderArgs()}
      </Box>
    );
  }

  return (
    <Box className="tm-agent-tool-card">
      <Box className="tm-agent-tool-head">
        <ButtonBase disableRipple className="tm-agent-tool-fold" onClick={() => setOpen(current => !current)}>
          <Box component="span" className="tm-agent-tool-chevron">
            {open ? <KeyboardArrowDown sx={{ fontSize: 16 }} /> : <KeyboardArrowRight sx={{ fontSize: 16 }} />}
          </Box>
          <Box component="span" className={resolveTitleClassName()}>
            {resolveTitle()}
          </Box>
          {isNotNil(args) ? (
            <Box component="span" className="tm-agent-tool-path" title={args.path}>
              {args.path}
            </Box>
          ) : null}
        </ButtonBase>
        {canOpenFile ? (
          <Box className="tm-agent-tool-actions">
            <ButtonBase disableRipple className="tm-agent-tool-icon-btn" title={t('agent.tool.documentOpen')} aria-label={t('agent.tool.documentOpen')} onClick={handleOpenFile}>
              <OpenInNew sx={{ fontSize: 15 }} />
            </ButtonBase>
            <ButtonBase disableRipple className="tm-agent-tool-icon-btn" title={t('agent.tool.documentReveal')} aria-label={t('agent.tool.documentReveal')} onClick={handleRevealFile}>
              <FolderOpen sx={{ fontSize: 15 }} />
            </ButtonBase>
          </Box>
        ) : null}
      </Box>
      {isNotNil(openError) ? <Box className="tm-agent-tool-cwd">{openError}</Box> : null}
      <Collapse in={open} unmountOnExit timeout={0}>
        <Box className="tm-agent-tool-body">
          {renderArgs()}
          {isNotNil(result) ? <AgentCodeBlock code={result} language="text" /> : null}
        </Box>
      </Collapse>
    </Box>
  );
}
