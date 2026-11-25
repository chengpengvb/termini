import { Ref } from 'react';
import { useI18n } from '@/common/i18n/useI18n.ts';
import type { ItemHandle } from '@/page/common/dto/agent/ItemHandle.ts';
import { ShellToolView } from '@/page/agent/main/body/tool/common/ShellToolView.tsx';
import type { ShellConflictView, ShellToolTitles } from '@/page/common/dto/agent/ToolView.ts';

interface ItemShellProcessProps {
  sessionId: string;
  ref?: Ref<ItemHandle>;
}

interface TtyWriteArgsRaw {
  ctrl?: string;
  chars?: string;
  command?: string;
  process_id: string | null;
}

function resolveTtyWriteInput(raw: TtyWriteArgsRaw): string | null {
  if (raw.ctrl) {
    return `Ctrl+${raw.ctrl.trim().toUpperCase()}`;
  }
  if (raw.command) {
    return raw.command;
  }
  if (raw.chars) {
    return raw.chars;
  }
  return null;
}

function resolveTtyWriteConflictArgs(args: unknown): ShellConflictView | null {
  if (!args) {
    return null;
  }
  const raw = args as TtyWriteArgsRaw;
  const command = resolveTtyWriteInput(raw);
  if (!command) {
    return null;
  }
  if (!raw.process_id) {
    return { command, detail: null };
  }
  return { command, detail: `process_id: ${raw.process_id}` };
}

function resolveNoConflictArgs(): ShellConflictView | null {
  return null;
}

export function ItemShellProcessWrite(props: ItemShellProcessProps) {
  const { t } = useI18n();
  const { sessionId, ref } = props;

  function resolveTitles(): ShellToolTitles {
    return {
      skipped: t('agent.tool.ttyWriteSkipped'),
      completed: t('agent.tool.ttyWriteCompleted'),
      running: t('agent.tool.ttyWriteRunning'),
      waitingConfirm: t('agent.tool.ttyWriteWaiting'),
      preparing: t('agent.tool.ttyWritePreparing'),
    };
  }

  return <ShellToolView sessionId={sessionId} ref={ref} titles={resolveTitles()} codeLanguage="text" resolveConflictArgs={resolveTtyWriteConflictArgs} />;
}

export function ItemShellProcessLog(props: ItemShellProcessProps) {
  const { t } = useI18n();
  const { sessionId, ref } = props;

  function resolveTitles(): ShellToolTitles {
    const running = t('agent.tool.processLogRunning');
    return {
      running,
      preparing: running,
      waitingConfirm: running,
      skipped: t('agent.tool.processLogCompleted'),
      completed: t('agent.tool.processLogCompleted'),
    };
  }

  return <ShellToolView sessionId={sessionId} ref={ref} titles={resolveTitles()} codeLanguage="text" resolveConflictArgs={resolveNoConflictArgs} />;
}

export function ItemShellProcessKill(props: ItemShellProcessProps) {
  const { t } = useI18n();
  const { sessionId, ref } = props;

  function resolveTitles(): ShellToolTitles {
    const running = t('agent.tool.processKillRunning');
    return {
      running,
      preparing: running,
      waitingConfirm: running,
      skipped: t('agent.tool.processKillCompleted'),
      completed: t('agent.tool.processKillCompleted'),
    };
  }

  return <ShellToolView sessionId={sessionId} ref={ref} titles={resolveTitles()} codeLanguage="text" resolveConflictArgs={resolveNoConflictArgs} />;
}

export function ItemShellProcessList(props: ItemShellProcessProps) {
  const { t } = useI18n();
  const { sessionId, ref } = props;

  function resolveTitles(): ShellToolTitles {
    const running = t('agent.tool.processListRunning');
    return {
      running,
      preparing: running,
      waitingConfirm: running,
      skipped: t('agent.tool.processListCompleted'),
      completed: t('agent.tool.processListCompleted'),
    };
  }

  return <ShellToolView sessionId={sessionId} ref={ref} titles={resolveTitles()} codeLanguage="text" resolveConflictArgs={resolveNoConflictArgs} />;
}
