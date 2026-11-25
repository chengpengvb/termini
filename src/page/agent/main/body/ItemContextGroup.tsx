import { useEffect, useState } from 'react';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { Box, ButtonBase, Collapse } from '@mui/material';
import type { AiChatItem } from '@/common/model/AiChatItem.ts';
import type { ItemHandle } from '@/page/common/dto/agent/ItemHandle.ts';
import { AI_TOOL_ID, resolveAiToolId } from '@/common/const/AiToolConst.ts';
import { KeyboardArrowDown, KeyboardArrowRight } from '@mui/icons-material';
import type { AgentQuestionAnswer } from '@/common/model/req/event/WsAgent.ts';
import { AgentMessageItem } from '@/page/agent/main/body/message/AgentMessageItem.tsx';
import { CONTEXT_GROUP_VARIANT, type ContextGroupVariant } from '@/page/common/dto/agent/ToolView.ts';

interface ItemContextGroupProps {
  active: boolean;
  sessionId: string;
  items: AiChatItem[];
  variant?: ContextGroupVariant;
  onAskAnswer?: (answer: AgentQuestionAnswer) => void;
  onRegisterHandle: (id: string, handle: ItemHandle | null) => void;
}

export function ItemContextGroup(props: ItemContextGroupProps) {
  const { tf } = useI18n();
  const followActive = props.variant === CONTEXT_GROUP_VARIANT.WRITE;
  const [open, setOpen] = useState(followActive && props.active);
  const { items, active, sessionId, variant = CONTEXT_GROUP_VARIANT.QUERY, onAskAnswer, onRegisterHandle } = props;

  useEffect(() => {
    if (!followActive) {
      return;
    }
    setOpen(active);
  }, [followActive, active]);

  function handleToggleOpen(): void {
    setOpen((current: boolean) => !current);
  }

  function resolveLabel(): string {
    if (variant === CONTEXT_GROUP_VARIANT.WRITE) {
      return tf('agent.tool.writeGroup', { n: items.length });
    }
    const allDerived = items.every((item: AiChatItem) => resolveAiToolId(item.context?.toolId) === AI_TOOL_ID.DERIVED_LIST);
    return allDerived ? tf('agent.tool.derivedGroup', { n: items.length }) : tf('agent.tool.contextGroup', { n: items.length });
  }

  const labelClassName = active ? 'tm-agent-fold-label tm-agent-shimmer' : 'tm-agent-fold-label';

  return (
    <Box className="tm-agent-msg tm-agent-msg-tool">
      <Box className="tm-agent-fold-row">
        <ButtonBase disableRipple className="tm-agent-fold-toggle" onClick={handleToggleOpen}>
          <Box component="span" className={labelClassName}>
            {resolveLabel()}
          </Box>
          <Box component="span" className="tm-agent-fold-chevron">
            {open ? <KeyboardArrowDown sx={{ fontSize: 16 }} /> : <KeyboardArrowRight sx={{ fontSize: 16 }} />}
          </Box>
        </ButtonBase>
        <Collapse in={open} unmountOnExit timeout={0}>
          <Box className="tm-agent-fold-body tm-agent-context-group-body">
            {items.map((item: AiChatItem) => (
              <AgentMessageItem key={item.id} item={item} sessionId={sessionId} onAskAnswer={onAskAnswer} onRegisterHandle={onRegisterHandle} />
            ))}
          </Box>
        </Collapse>
      </Box>
    </Box>
  );
}
