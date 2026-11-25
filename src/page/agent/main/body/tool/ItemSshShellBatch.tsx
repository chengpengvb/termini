import { Ref } from 'react';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { useDeviceService } from '@/common/context/useAppHooks.tsx';
import type { ItemHandle } from '@/page/common/dto/agent/ItemHandle.ts';
import { resolveDeviceListDetail } from '@/page/common/util/deviceDetail.ts';
import { ShellToolView } from '@/page/agent/main/body/tool/common/ShellToolView.tsx';
import type { ShellConflictView, ShellToolTitles } from '@/page/common/dto/agent/ToolView.ts';

interface ItemSshShellBatchProps {
  sessionId: string;
  ref?: Ref<ItemHandle>;
}

interface SshShellBatchConflictArgsRaw {
  command: string;
  device_ids: string[];
}

export function ItemSshShellBatch(props: ItemSshShellBatchProps) {
  const { t } = useI18n();
  const { sessionId, ref } = props;
  const deviceService = useDeviceService();

  function resolveConflictArgs(args: unknown): ShellConflictView | null {
    if (!args) {
      return null;
    }
    const raw = args as SshShellBatchConflictArgsRaw;
    if (!raw.command) {
      return null;
    }
    return {
      command: raw.command,
      detail: resolveDeviceListDetail(deviceService, raw.device_ids),
    };
  }

  function resolveTitles(): ShellToolTitles {
    return {
      skipped: t('agent.tool.sshSkipped'),
      completed: t('agent.tool.sshCompleted'),
      running: t('agent.tool.sshRunning'),
      waitingConfirm: t('agent.tool.sshWaiting'),
      preparing: t('agent.tool.sshPreparing'),
    };
  }

  return <ShellToolView sessionId={sessionId} ref={ref} titles={resolveTitles()} codeLanguage="bash" deviceBatch resolveConflictArgs={resolveConflictArgs} />;
}
