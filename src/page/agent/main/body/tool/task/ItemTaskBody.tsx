import { Box, Collapse } from '@mui/material';
import { isNil, isNotNil } from '@/common/util/StringUtil.ts';
import type { AiChatItem } from '@/common/model/AiChatItem.ts';
import type { LegacyStep } from '@/page/common/dto/agent/ToolView.ts';
import type { ItemHandle } from '@/page/common/dto/agent/ItemHandle.ts';
import { AgentMessageItem } from '@/page/agent/main/body/message/AgentMessageItem.tsx';

interface ItemTaskBodyProps {
  open: boolean;
  note: string | null;
  sessionId: string;
  legacySteps: LegacyStep[];
  nestedRoots: AiChatItem[];
  onRegisterNested: (id: string, handle: ItemHandle | null) => void;
}

export function ItemTaskBody(props: ItemTaskBodyProps) {
  const { open, note, sessionId, legacySteps, nestedRoots, onRegisterNested } = props;

  return (
    <Collapse in={open} unmountOnExit timeout={0}>
      <Box className="tm-agent-tool-body">
        {nestedRoots.length > 0 ? (
          <Box className="tm-agent-tool-task-nested">
            {nestedRoots.map((root: AiChatItem) => (
              <Box key={root.id} className="tm-agent-msg-row">
                <AgentMessageItem item={root} sessionId={sessionId} canEdit={false} onRegisterHandle={onRegisterNested} />
              </Box>
            ))}
          </Box>
        ) : null}
        {legacySteps.length > 0 ? (
          <Box className="tm-agent-tool-task-steps">
            {legacySteps.map((step: LegacyStep) => (
              <Box key={step.subCallId} className={step.ok === false ? 'tm-agent-tool-task-step is-failed' : 'tm-agent-tool-task-step'}>
                <Box component="span" className="tm-agent-tool-task-step-tool">
                  {step.toolId}
                </Box>
                <Box component="span" className="tm-agent-tool-task-step-args" title={step.args}>
                  {step.args}
                </Box>
                <Box component="span" className="tm-agent-tool-task-step-cost">
                  {isNil(step.elapsedMs) ? '' : `${step.elapsedMs}ms`}
                </Box>
              </Box>
            ))}
          </Box>
        ) : null}
        {isNotNil(note) ? <Box className="tm-agent-tool-task-note">{note}</Box> : null}
      </Box>
    </Collapse>
  );
}
