import { Box } from '@mui/material';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { isNil } from '@/common/util/StringUtil.ts';
import { useElapsed } from '@/page/common/hooks/useElapsed.ts';
import type { AiChatItem } from '@/common/model/AiChatItem.ts';
import type { ItemHandle } from '@/page/common/dto/agent/ItemHandle.ts';
import { useTaskGroupState } from '@/page/common/util/taskPhaseStore.ts';
import { AgentMessageItem } from '@/page/agent/main/body/message/AgentMessageItem.tsx';

interface ItemTaskGroupProps {
  items: AiChatItem[];
  sessionId: string;
  onRegisterHandle: (id: string, handle: ItemHandle | null) => void;
}

export function ItemTaskGroup(props: ItemTaskGroupProps) {
  const { tf } = useI18n();
  const { items, sessionId, onRegisterHandle } = props;
  const state = useTaskGroupState(items.map((item: AiChatItem) => item.id));
  const startedAt = state.startedAt ?? items[0]?.timestamp ?? null;
  const elapsed = useElapsed(startedAt, state.running);

  return (
    <Box className="tm-agent-tool-task-group">
      {items.map((item: AiChatItem) => (
        <AgentMessageItem key={item.id} item={item} sessionId={sessionId} onRegisterHandle={onRegisterHandle} />
      ))}
      {state.running && !isNil(elapsed) ? (
        <Box className="tm-agent-tool-task-group-wait">
          <Box component="span" className="tm-agent-tool-task-group-wait-text tm-agent-shimmer">
            {tf('agent.tool.taskGroupWaiting', { elapsed })}
          </Box>
        </Box>
      ) : null}
    </Box>
  );
}
