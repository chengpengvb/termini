import { Ref, useState } from 'react';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { isNil, isNotNil } from '@/common/util/StringUtil.ts';
import type { AiChatItem } from '@/common/model/AiChatItem.ts';
import { Box, Button, ButtonBase, Collapse } from '@mui/material';
import { useAgentService } from '@/common/context/useAppHooks.tsx';
import { useItemHandle } from '@/page/common/hooks/useItemHandle.ts';
import { AgentCodeBlock } from '@/page/agent/common/AgentCodeBlock.tsx';
import type { ItemHandle } from '@/page/common/dto/agent/ItemHandle.ts';
import { KeyboardArrowDown, KeyboardArrowRight } from '@mui/icons-material';
import { RESP_EVENT_AGENT } from '@/common/model/resp/event/RespEventAgent.ts';

interface ItemMailSendProps {
  sessionId: string;
  ref?: Ref<ItemHandle>;
}

interface MailSendArgs {
  to: string[];
  body: string;
  subject: string;
}

const CANCELLED_RESULT = 'exit_code=cancelled';

export function ItemMailSend(props: ItemMailSendProps) {
  const { t } = useI18n();
  const { sessionId, ref } = props;
  const agentService = useAgentService();
  const [open, setOpen] = useState(false);
  const [done, setDone] = useState(false);
  const [cancelled, setCancelled] = useState(false);
  const [reqId, setReqId] = useState<string | null>(null);
  const [runId, setRunId] = useState<string | null>(null);
  const [callId, setCallId] = useState<string | null>(null);
  const [result, setResult] = useState<string | null>(null);
  const [args, setArgs] = useState<MailSendArgs | null>(null);

  function resolveArgs(raw: unknown): MailSendArgs | null {
    if (isNil(raw)) {
      return null;
    }
    const record = raw as MailSendArgs;
    if (!Array.isArray(record.to) || isNil(record.subject) || isNil(record.body) || record.to.length === 0) {
      return null;
    }
    return { to: record.to, subject: record.subject, body: record.body };
  }

  function resolveTitle(): string {
    if (cancelled) {
      return t('agent.tool.mailSendCancelled');
    }
    if (done) {
      return t('agent.tool.mailSendCompleted');
    }
    return t('agent.tool.mailSendRunning');
  }

  function resolveTitleClassName(): string {
    if (done || cancelled) {
      return 'tm-agent-tool-title';
    }
    return 'tm-agent-tool-title tm-agent-shimmer';
  }

  function canCancel(): boolean {
    return !done && !cancelled && isNotNil(callId) && isNotNil(reqId) && isNotNil(runId);
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

  function renderArgs() {
    if (isNil(args)) {
      return null;
    }
    const lines = [`${t('agent.tool.mailSendTo')}: ${args.to.join(', ')}`, `${t('agent.tool.mailSendSubject')}: ${args.subject}`, '', args.body];
    return (
      <Box className="tm-agent-tool-command">
        <AgentCodeBlock code={lines.join('\n')} language="html" />
      </Box>
    );
  }

  function onEvent(eventItem: AiChatItem): void {
    if (eventItem.type === RESP_EVENT_AGENT.ON_TOOL_CALL) {
      setReqId(eventItem.context.reqId);
      setRunId(eventItem.context.runId);
      setCallId(eventItem.context.callId);
      const parsed = resolveArgs(eventItem.context.args);
      if (isNotNil(parsed)) {
        setArgs(parsed);
      }
      return;
    }
    if (eventItem.type === RESP_EVENT_AGENT.ON_TOOL_CALL_DONE) {
      const next = eventItem.context.result;
      setResult(next);
      if (next === CANCELLED_RESULT) {
        setCancelled(true);
      }
      setDone(true);
      return;
    }
    if (eventItem.type === RESP_EVENT_AGENT.ON_DONE || eventItem.type === RESP_EVENT_AGENT.ON_CANCELLED) {
      if (eventItem.type === RESP_EVENT_AGENT.ON_CANCELLED) {
        setCancelled(true);
      }
      setDone(true);
    }
  }

  useItemHandle(ref, onEvent);

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
          {isNotNil(args) ? (
            <Box component="span" className="tm-agent-tool-path" title={args.subject}>
              {args.subject}
            </Box>
          ) : null}
        </ButtonBase>
        {canCancel() ? (
          <Button disableRipple className="tm-btn tm-btn-ghost tm-btn-compact" onClick={handleCancel}>
            {t('agent.tool.cancel')}
          </Button>
        ) : null}
      </Box>
      <Collapse in={open} unmountOnExit timeout={0}>
        <Box className="tm-agent-tool-body">
          {renderArgs()}
          {isNotNil(result) ? <AgentCodeBlock code={result} language="text" /> : null}
        </Box>
      </Collapse>
    </Box>
  );
}
