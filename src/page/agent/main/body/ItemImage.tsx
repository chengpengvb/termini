import { Box } from '@mui/material';
import { Ref, useState } from 'react';
import type { AiChatItem } from '@/common/model/AiChatItem.ts';
import { FilePreview } from '@/page/agent/common/FilePreview.tsx';
import { useItemHandle } from '@/page/common/hooks/useItemHandle.ts';
import type { ItemHandle } from '@/page/common/dto/agent/ItemHandle.ts';
import { RESP_EVENT_AGENT } from '@/common/model/resp/event/RespEventAgent.ts';

interface ItemImageProps {
  item: AiChatItem;
  ref?: Ref<ItemHandle>;
}

export function ItemImage(props: ItemImageProps) {
  const { ref, item } = props;
  const [path, setPath] = useState<string | null>(() => item.context.path ?? null);

  function onEvent(eventItem: AiChatItem): void {
    if (eventItem.type !== RESP_EVENT_AGENT.ON_IMAGE) {
      return;
    }
    const nextPath = eventItem.context.path;
    if (!nextPath) {
      return;
    }
    setPath(nextPath);
  }

  useItemHandle(ref, onEvent);
  if (!path) {
    return null;
  }

  return (
    <Box className="tm-agent-msg tm-agent-msg-assistant">
      <FilePreview key={path} path={path}/>
    </Box>
  );
}
