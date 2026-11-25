import { Ref, useState } from 'react';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { Box, ButtonBase, Collapse } from '@mui/material';
import { isNil, isNotNil } from '@/common/util/StringUtil.ts';
import type { AiChatItem } from '@/common/model/AiChatItem.ts';
import { useItemHandle } from '@/page/common/hooks/useItemHandle.ts';
import type { ItemHandle } from '@/page/common/dto/agent/ItemHandle.ts';
import { KeyboardArrowDown, KeyboardArrowRight } from '@mui/icons-material';
import { RESP_EVENT_AGENT } from '@/common/model/resp/event/RespEventAgent.ts';

interface ItemCodeSemanticProps {
  sessionId: string;
  ref?: Ref<ItemHandle>;
}

interface IndexProgress {
  done?: number;
  stage: string;
  files?: number;
  total?: number;
  reuse?: number;
  chunks?: number;
  pending?: number;
}

function formatCount(value?: number): string {
  return (value ?? 0).toLocaleString();
}

export function ItemCodeSemantic(props: ItemCodeSemanticProps) {
  const { ref } = props;
  const { t, tf } = useI18n();
  const [open, setOpen] = useState(false);
  const [done, setDone] = useState(false);
  const [query, setQuery] = useState<string | null>(null);
  const [result, setResult] = useState<string | null>(null);
  const [progress, setProgress] = useState<IndexProgress | null>(null);

  function parseProgress(delta: string): IndexProgress | null {
    let latest: IndexProgress | null = null;
    for (const line of delta.split('\n')) {
      const text = line.trim();
      if (text.length === 0) {
        continue;
      }
      try {
        latest = JSON.parse(text) as IndexProgress;
      } catch (e) {
        console.warn(`[code-semantic] parse error: ${e}`);
      }
    }
    return latest;
  }

  function resolveTitle(): string {
    if (done) {
      return t('agent.tool.codeSemanticDone');
    }
    if (isNil(progress)) {
      return t('agent.tool.codeSemanticRunning');
    }
    if (progress.stage === 'scan') {
      return tf('agent.tool.codeSemanticScan', { n: progress.files ?? 0 });
    }
    if (progress.stage === 'chunk') {
      return tf('agent.tool.codeSemanticChunk', { total: progress.total ?? 0, reuse: progress.reuse ?? 0, pending: progress.pending ?? 0 });
    }
    if (progress.stage === 'embed') {
      return tf('agent.tool.codeSemanticEmbed', { done: formatCount(progress.done), total: formatCount(progress.total) });
    }
    if (progress.stage === 'save') {
      return tf('agent.tool.codeSemanticSave', { n: progress.chunks ?? 0 });
    }
    return t('agent.tool.codeSemanticRunning');
  }

  function resolvePercent(): number | null {
    if (done || isNil(progress) || progress.stage !== 'embed') {
      return null;
    }
    const total = progress.total ?? 0;
    if (total <= 0) {
      return null;
    }
    return Math.min(100, Math.round(((progress.done ?? 0) * 100) / total));
  }

  function handleToggleOpen(): void {
    setOpen((current: boolean) => !current);
  }

  function onEvent(eventItem: AiChatItem): void {
    if (eventItem.type === RESP_EVENT_AGENT.ON_TOOL_CALL) {
      if (eventItem.context.args?.query) {
        setQuery(eventItem.context.args.query);
      }
      return;
    }
    if (eventItem.type === RESP_EVENT_AGENT.ON_TOOL_CALL_CHUNK) {
      const delta = eventItem.context.delta;
      if (isNil(delta)) {
        return;
      }
      const next = parseProgress(delta);
      if (isNotNil(next)) {
        setProgress(next);
      }
      return;
    }
    if (eventItem.type === RESP_EVENT_AGENT.ON_TOOL_CALL_DONE) {
      setResult(eventItem.context.result);
      setDone(true);
      return;
    }
    if (eventItem.type === RESP_EVENT_AGENT.ON_DONE || eventItem.type === RESP_EVENT_AGENT.ON_CANCELLED) {
      setDone(true);
    }
  }

  useItemHandle(ref, onEvent);

  const percent = resolvePercent();

  // 与读取文件、写入等工具行保持同一形态: 卡片边框 + 常显 chevron + 右侧目标(这里是查询词)。
  // 之前用的是无边框的 tm-agent-fold-row, chevron 还要 hover 才现, 几条挤在一起像一堆散文字。
  function renderToggle() {
    return (
      <ButtonBase disableRipple className="tm-agent-tool-fold" onClick={handleToggleOpen}>
        <Box component="span" className="tm-agent-tool-chevron">
          {open ? <KeyboardArrowDown sx={{ fontSize: 16 }} /> : <KeyboardArrowRight sx={{ fontSize: 16 }} />}
        </Box>
        <Box component="span" className={done ? 'tm-agent-tool-title' : 'tm-agent-tool-title tm-agent-shimmer'}>
          {resolveTitle()}
        </Box>
        {isNotNil(query) ? (
          <Box component="span" className="tm-agent-tool-path" title={query}>
            {query}
          </Box>
        ) : null}
      </ButtonBase>
    );
  }

  return (
    <Box className="tm-agent-tool-card">
      <Box className="tm-agent-tool-head">{renderToggle()}</Box>
      {percent === null ? null : (
        <Box className="tm-agent-progress-shell">
          <Box className="tm-agent-progress-track">
            <Box className="tm-agent-progress-fill" style={{ width: `${percent}%` }} />
          </Box>
          <Box component="span" className="tm-agent-progress-value">
            {percent}%
          </Box>
        </Box>
      )}
      <Collapse in={open} unmountOnExit timeout={0}>
        <Box className="tm-agent-tool-body">
          {isNotNil(result) ? (
            <Box component="pre" className="tm-agent-shell-output-static">
              {result}
            </Box>
          ) : null}
        </Box>
      </Collapse>
    </Box>
  );
}
