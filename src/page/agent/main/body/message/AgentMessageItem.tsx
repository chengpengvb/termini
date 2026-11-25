import { Box } from '@mui/material';
import type { ReactNode } from 'react';
import { memo, useCallback } from 'react';
import { isNil } from '@/common/util/StringUtil.ts';
import { ItemText } from '@/page/agent/main/body/ItemText.tsx';
import { ItemUser } from '@/page/agent/main/body/ItemUser.tsx';
import type { AiChatItem } from '@/common/model/AiChatItem.ts';
import { ItemError } from '@/page/agent/main/body/ItemError.tsx';
import { ItemImage } from '@/page/agent/main/body/ItemImage.tsx';
import { ItemTools } from '@/page/agent/main/body/tool/ItemTools.tsx';
import type { ItemHandle } from '@/page/common/dto/agent/ItemHandle.ts';
import { ItemReasoning } from '@/page/agent/main/body/ItemReasoning.tsx';
import { RESP_EVENT_AGENT } from '@/common/model/resp/event/RespEventAgent.ts';
import type { AgentQuestionAnswer } from '@/common/model/req/event/WsAgent.ts';

interface AgentMessageItemProps {
  item: AiChatItem;
  sessionId: string;
  canEdit?: boolean;
  editing?: boolean;
  onAskAnswer?: (answer: AgentQuestionAnswer) => void;
  onRevertEdit?: (itemId: string, message: string) => void;
  onRegisterHandle: (id: string, handle: ItemHandle | null) => void;
}

export const AgentMessageItem = memo(function AgentMessageItem(props: AgentMessageItemProps) {
  const { item, sessionId, canEdit, editing, onAskAnswer, onRevertEdit, onRegisterHandle } = props;

  const registerHandle = useCallback((handle: ItemHandle | null): void => {
    onRegisterHandle(item.id, handle);
  }, [item.id, onRegisterHandle]);

  function renderContent(): ReactNode {
    if (item.type === RESP_EVENT_AGENT.ON_TOOL_CALL) {
      return <ItemTools itemId={item.id} sessionId={sessionId} onAskAnswer={onAskAnswer} ref={registerHandle} />;
    }
    if (item.type === RESP_EVENT_AGENT.ON_ERROR) {
      return <ItemError item={item} ref={registerHandle} />;
    }
    if (item.type === RESP_EVENT_AGENT.ON_USER) {
      return <ItemUser item={item} canEdit={canEdit} editing={editing} onRevertEdit={onRevertEdit} ref={registerHandle} />;
    }
    const Component = (
      {
        [RESP_EVENT_AGENT.ON_TEXT]: ItemText,
        [RESP_EVENT_AGENT.ON_TEXT_FINAL]: ItemText,
        [RESP_EVENT_AGENT.ON_IMAGE]: ItemImage,
        [RESP_EVENT_AGENT.ON_REASONING]: ItemReasoning,
        [RESP_EVENT_AGENT.ON_REASONING_FINAL]: ItemReasoning,
      } as Record<number, typeof ItemText | typeof ItemReasoning | typeof ItemImage>
    )[item.type];
    if (isNil(Component)) {
      return null;
    }
    return <Component item={item} ref={registerHandle} />;
  }

  return <Box className="tm-agent-msg-item">{renderContent()}</Box>;
});
