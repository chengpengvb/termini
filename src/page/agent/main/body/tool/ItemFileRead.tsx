import { Ref } from 'react';
import { useI18n } from '@/common/i18n/useI18n.ts';
import type { ItemHandle } from '@/page/common/dto/agent/ItemHandle.ts';
import { openFileEditTab } from '@/page/common/util/fileEditOpenUtil.ts';
import type { FileReadToolTitles } from '@/page/common/dto/agent/ToolView.ts';
import { FileReadToolView } from '@/page/agent/main/body/tool/common/FileReadToolView.tsx';
import { useSystemUtilService, useTabManagerService } from '@/common/context/useAppHooks.tsx';

interface ItemFileReadProps {
  sessionId: string;
  ref?: Ref<ItemHandle>;
}

export function ItemFileRead(props: ItemFileReadProps) {
  const { ref } = props;
  const { t, tf } = useI18n();
  const tabManagerService = useTabManagerService();
  const systemUtilService = useSystemUtilService();

  function resolveTitles(): FileReadToolTitles {
    return {
      reading: t('agent.tool.fileReadRunning'),
      done: t('agent.tool.fileReadDone'),
      batchReading: (count: number) => tf('agent.tool.fileReadBatchRunning', { count }),
      batchDone: (count: number) => tf('agent.tool.fileReadBatchDone', { count }),
    };
  }

  function handleOpenPath(path: string): void {
    openFileEditTab(tabManagerService, systemUtilService, path);
  }

  return <FileReadToolView ref={ref} titles={resolveTitles()} onOpenPath={handleOpenPath} />;
}
