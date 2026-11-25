import { Box, ButtonBase } from '@mui/material';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { ArrowUpward, Stop } from '@mui/icons-material';
import type { AiProvide } from '@/common/model/AiProvide.ts';
import { usePlatformMobile } from '@/common/context/usePlatformMobile';
import type { AttachedImage } from '@/page/common/util/imageAttach.ts';
import type { ChatReference } from '@/common/model/dto/ChatReference.ts';
import { ModelSelect } from '@/page/common/components/model/ModelSelect.tsx';
import { AddMenu } from '@/page/agent/main/input/components/tool/AddMenu.tsx';
import { ConfirmModeSelect } from '@/page/agent/main/input/components/tool/ConfirmModeSelect.tsx';
import { ContextUsageBadge } from '@/page/agent/main/input/components/tool/ContextUsageBadge.tsx';
import { ThinkingLevelSelect } from '@/page/agent/main/input/components/tool/ThinkingLevelSelect.tsx';

interface ToolbarProps {
  running: boolean;
  canSend: boolean;
  onSend: () => void;
  onStop: () => void;
  confirmMode: number;
  remainingSlots: number;
  chatId: string | null;
  selectedId: string | null;
  thinkingLevels: string[];
  thinkingLevel: string | null;
  onSelectModel: (provide: AiProvide) => void;
  onSelectConfirmMode: (mode: number) => void;
  onSelectThinkingLevel: (level: string) => void;
  onAddImages: (images: AttachedImage[]) => void;
  onInsertReference: (ref: ChatReference) => void;
}

export function Toolbar(props: ToolbarProps) {
  const { t } = useI18n();
  const mobile = usePlatformMobile();
  const { running, canSend, confirmMode, remainingSlots, chatId, selectedId, thinkingLevels, thinkingLevel, onSend, onStop, onAddImages, onSelectModel, onSelectConfirmMode, onSelectThinkingLevel, onInsertReference } = props;
  return (
    <Box className={mobile ? 'tm-agent-input-toolbar is-mobile' : 'tm-agent-input-toolbar'}>
      <Box className="tm-agent-input-tools">
        <AddMenu remainingSlots={remainingSlots} onAddImages={onAddImages} onInsertReference={onInsertReference} />
        <ConfirmModeSelect mode={confirmMode} onSelect={onSelectConfirmMode} />
        <ThinkingLevelSelect levels={thinkingLevels} level={thinkingLevel} onSelect={onSelectThinkingLevel} />
      </Box>
      <Box className="tm-agent-input-actions">
        <ModelSelect selectedId={selectedId} onSelect={onSelectModel} variant="toolbar" autoSelect />
        <ContextUsageBadge chatId={chatId} provideId={selectedId} running={running} />
        {running && (mobile || !canSend) ? (
          <ButtonBase
            disableRipple
            onPointerDown={event => {
              event.preventDefault();
              onStop();
            }}
            onClick={onStop}
            aria-label={t('agent.toolbar.stop')}
            className="tm-agent-input-send is-stop"
          >
            <Stop sx={{ fontSize: 16 }} />
          </ButtonBase>
        ) : (
          <ButtonBase disableRipple disabled={!canSend} onClick={onSend} aria-label={running ? t('agent.toolbar.queueSend') : t('agent.toolbar.send')} className={running ? 'tm-agent-input-send is-queue' : 'tm-agent-input-send'}>
            <ArrowUpward sx={{ fontSize: 16 }} />
          </ButtonBase>
        )}
      </Box>
    </Box>
  );
}
