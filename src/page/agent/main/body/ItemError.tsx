import { Box } from '@mui/material';
import { Ref, useState } from 'react';
import { isNil, isNotNil } from '@/common/util/StringUtil.ts';
import type { AiChatItem } from '@/common/model/AiChatItem.ts';
import { useItemHandle } from '@/page/common/hooks/useItemHandle.ts';
import type { ItemHandle } from '@/page/common/dto/agent/ItemHandle.ts';
import { resolveResultMessage } from '@/common/model/resp/ResultModel.ts';
import { RESP_EVENT_AGENT } from '@/common/model/resp/event/RespEventAgent.ts';

interface ItemErrorProps {
  item: AiChatItem;
  ref?: Ref<ItemHandle>;
}

export function ItemError(props: ItemErrorProps) {
  const { ref } = props;
  const [content, setContent] = useState<string | null>(null);

  function resolveMessage(nextCode: number | null, error: string): string | null {
    if (isNotNil(nextCode)) {
      return resolveResultMessage(nextCode);
    }
    if (!error) {
      return null;
    }
    return error;
  }

  function applyError(eventItem: AiChatItem): void {
    const nextCodeRaw = eventItem.context.code;
    let nextCode: number | null = null;
    if (isNotNil(nextCodeRaw)) {
      nextCode = nextCodeRaw;
    }
    let error = '';
    if (eventItem.context.error) {
      error = eventItem.context.error;
    }
    setContent(resolveMessage(nextCode, error));
  }

  function onEvent(eventItem: AiChatItem): void {
    if (eventItem.type !== RESP_EVENT_AGENT.ON_ERROR) {
      return;
    }
    applyError(eventItem);
  }

  useItemHandle(ref, onEvent);

  if (isNil(content)) {
    return null;
  }

  return (
    <Box className="tm-agent-msg tm-agent-msg-notice">
      <Box className="tm-agent-msg-notice-text">{content}</Box>
    </Box>
  );
}
