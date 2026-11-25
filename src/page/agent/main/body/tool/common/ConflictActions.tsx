import { Box, Button } from '@mui/material';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { isNil } from '@/common/util/StringUtil.ts';
import { useRef, useState, type PointerEvent } from 'react';
import { useAgentService } from '@/common/context/useAppHooks.tsx';
import { ArrowDropDown, DoneAll, History } from '@mui/icons-material';
import { AGENT_ANSWER_CODE } from '@/common/model/req/event/WsAgent.ts';
import { CommonMenu, type CommonMenuPosition } from '@/common/components/menu/CommonMenu.tsx';

const MENU_ITEM = {
  ALLOW_REMEMBER: 1,
  ALLOW_SESSION_PREFIX: 2,
} as const;

interface ConflictActionsProps {
  sessionId: string;
  callId: string | null;
  sessionPrefix?: string | null; // 后端按配置规则算出的命令前缀, 为空表示按同工具同设备放行
}

export function ConflictActions(props: ConflictActionsProps) {
  const { t, tf } = useI18n();
  const agentService = useAgentService();
  const { sessionId, callId, sessionPrefix } = props;
  const splitRef = useRef<HTMLDivElement | null>(null);
  const [menuPosition, setMenuPosition] = useState<CommonMenuPosition | null>(null);
  const menuOpen = menuPosition !== null;
  const sessionLabel = isNil(sessionPrefix) || sessionPrefix.length === 0 ? t('agent.tool.allowSessionSame') : tf('agent.tool.allowSessionPrefix', { prefix: sessionPrefix });

  function handleAnswer(code: number): void {
    if (isNil(callId)) {
      throw new Error('tool answer missing callId');
    }
    agentService.handleToolAnswer(sessionId, {
      data: { code, callId },
    });
  }

  function press(event: PointerEvent<HTMLButtonElement>, action: () => void): void {
    event.preventDefault();
    action();
  }

  function handleAllow(): void {
    handleAnswer(AGENT_ANSWER_CODE.ALLOW);
  }

  function handleSkip(): void {
    handleAnswer(AGENT_ANSWER_CODE.REJECT);
  }

  function handleAllowRemember(): void {
    handleAnswer(AGENT_ANSWER_CODE.ALLOW_REMEMBER);
  }

  function handleAllowSessionPrefix(): void {
    handleAnswer(AGENT_ANSWER_CODE.ALLOW_SESSION_PREFIX);
  }

  function handleOpenMenu(): void {
    const split = splitRef.current;
    if (!split) {
      return;
    }
    const rect = split.getBoundingClientRect();
    setMenuPosition({ top: rect.bottom + 4, left: rect.left });
  }

  function handleCloseMenu(): void {
    setMenuPosition(null);
  }

  return (
    <Box className="tm-agent-tool-actions">
      <Box ref={splitRef} className={menuOpen ? 'tm-edit-split-btn is-open' : 'tm-edit-split-btn'}>
        <Button disableRipple className="tm-btn tm-btn-primary tm-btn-compact tm-edit-split-btn-main" onPointerDown={event => press(event, handleAllow)} onClick={handleAllow}>
          {t('agent.tool.allow')}
        </Button>
        <Button disableRipple className="tm-btn tm-btn-primary tm-btn-compact tm-edit-split-btn-more" onPointerDown={event => press(event, handleOpenMenu)} onClick={handleOpenMenu} aria-label={t('agent.tool.allowMoreAria')}>
          <ArrowDropDown sx={{ fontSize: 16 }} />
        </Button>
      </Box>
      <Button disableRipple className="tm-btn tm-btn-secondary tm-btn-compact" onPointerDown={event => press(event, handleSkip)} onClick={handleSkip}>
        {t('agent.tool.skip')}
      </Button>
      <CommonMenu
        position={menuPosition}
        onClose={handleCloseMenu}
        items={[
          { key: MENU_ITEM.ALLOW_SESSION_PREFIX, text: sessionLabel, icon: <History />, onClick: handleAllowSessionPrefix },
          { key: MENU_ITEM.ALLOW_REMEMBER, text: t('agent.tool.allowRemember'), icon: <DoneAll />, onClick: handleAllowRemember },
        ]}
      />
    </Box>
  );
}
