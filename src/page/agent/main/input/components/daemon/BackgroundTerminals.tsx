import { Box, ButtonBase } from '@mui/material';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { Fragment, useEffect, useRef, useState } from 'react';
import type { AiChatItem } from '@/common/model/AiChatItem.ts';
import { ShellPage } from '@/page/protocol/shell/ShellPage.tsx';
import { AgentShellResult } from '@/page/agent/common/AgentShellResult.tsx';
import type { RespSocketContext } from '@/common/model/resp/RespWsContext.ts';
import { RESP_EVENT_AGENT } from '@/common/model/resp/event/RespEventAgent.ts';
import { KeyboardArrowDown, KeyboardArrowRight, Loop, Terminal } from '@mui/icons-material';
import { resolveShellIcon, resolveShellKind, resolveShellLabel } from '@/page/common/util/shellUtil.ts';
import { useAgentService, useAiAgentApiService, useShellService, useTabManagerService } from '@/common/context/useAppHooks.tsx';
import type { RespAgentShellDaemon, RespAgentShellDaemonChunk, RespAgentShellDaemonRow } from '@/common/model/resp/RespAgent.ts';

interface BackgroundTerminalsProps {
  chatId: string | null;
}

export function BackgroundTerminals(props: BackgroundTerminalsProps) {
  const { chatId } = props;
  const { t, tf } = useI18n();
  const chatIdRef = useRef(chatId);
  const shellService = useShellService();
  const agentService = useAgentService();
  const aiAgentApiService = useAiAgentApiService();
  const tabManagerService = useTabManagerService();
  const openedTtyIdsRef = useRef<Set<string>>(new Set());
  const [openIds, setOpenIds] = useState<string[]>([]);
  const [expanded, setExpanded] = useState<boolean>(false);
  const [stoppingIds, setStoppingIds] = useState<string[]>([]);
  const [daemons, setDaemons] = useState<RespAgentShellDaemonRow[]>([]);

  chatIdRef.current = chatId;

  function resolveTitle(row: RespAgentShellDaemonRow): string {
    return row.command.split('\n')[0].trim();
  }

  function canExpand(row: RespAgentShellDaemonRow): boolean {
    return !row.tty && row.shellId !== null;
  }

  function isOpen(row: RespAgentShellDaemonRow): boolean {
    return openIds.includes(row.processId);
  }

  function handleToggleRow(row: RespAgentShellDaemonRow): void {
    setOpenIds(prev => (prev.includes(row.processId) ? prev.filter(id => id !== row.processId) : [...prev, row.processId]));
  }

  function appendOutput(chunk: RespAgentShellDaemonChunk): void {
    setDaemons(prev => prev.map(row => (row.processId === chunk.processId ? { ...row, output: row.output + chunk.delta } : row)));
  }

  function handleOpenTerminal(row: RespAgentShellDaemonRow): void {
    shellService.discover().then(items => {
      const shellDescr = items.find(item => item.shell_id === row.shellId);
      if (!shellDescr) {
        console.warn(`[daemon] shell not found: ${row.shellId}`);
        return;
      }
      const kind = resolveShellKind(shellDescr.shell_id);
      tabManagerService.openTab({
        panes: [
          {
            tabBar: {
              canClose: true,
              icon: resolveShellIcon(kind),
              title: `${resolveShellLabel(kind)} · ${resolveTitle(row)}`,
            },
            content: <ShellPage shellDescr={shellDescr} attachProcessId={row.processId} />,
          },
        ],
      });
    });
  }

  function openNewTtyTerminals(rows: RespAgentShellDaemonRow[]): void {
    for (const row of rows) {
      if (!row.tty || openedTtyIdsRef.current.has(row.processId)) {
        continue;
      }
      openedTtyIdsRef.current.add(row.processId);
      handleOpenTerminal(row);
    }
  }

  function resolveHeader(): string {
    if (daemons.length === 1) {
      return t('agent.daemon.titleOne');
    }
    return tf('agent.daemon.titleMany', { count: daemons.length });
  }

  function handleToggle(): void {
    setExpanded(prev => !prev);
  }

  function applyDaemons(rows: RespAgentShellDaemonRow[]): void {
    setDaemons(rows);
    setOpenIds(prev => prev.filter(id => rows.some(row => row.processId === id)));
    setStoppingIds(prev => prev.filter(id => rows.some(row => row.processId === id)));
  }

  function loadDaemons(targetChatId: string): void {
    aiAgentApiService
      .listShellDaemons(targetChatId)
      .then(rows => {
        if (chatIdRef.current !== targetChatId) {
          return;
        }
        for (const row of rows) {
          if (row.tty) {
            openedTtyIdsRef.current.add(row.processId);
          }
        }
        applyDaemons(rows);
      })
      .catch(err => {
        console.error('[daemon] list error', err);
      });
  }

  function handleStop(row: RespAgentShellDaemonRow): void {
    if (!chatId) {
      return;
    }
    setStoppingIds(prev => [...prev, row.processId]);
    aiAgentApiService.stopShellDaemon({ chatId, processId: row.processId }).catch(err => {
      console.error('[daemon] stop error', err);
      setStoppingIds(prev => prev.filter(id => id !== row.processId));
    });
  }

  useEffect(() => {
    setDaemons([]);
    setStoppingIds([]);
    if (!chatId) {
      return;
    }
    loadDaemons(chatId);
    return agentService.addResyncListener(chatId, () => loadDaemons(chatId));
  }, [agentService, aiAgentApiService, chatId]);

  useEffect(() => {
    if (!chatId) {
      return;
    }
    return agentService.addEventListener(chatId, (arg: { context: RespSocketContext }) => {
      const item = arg.context.payload as AiChatItem;
      if (!item) {
        return;
      }
      if (item.type === RESP_EVENT_AGENT.ON_SHELL_DAEMON_CHUNK) {
        const chunk = item.context as RespAgentShellDaemonChunk;
        if (chunk.chatId === chatId) {
          appendOutput(chunk);
        }
        return;
      }
      if (item.type !== RESP_EVENT_AGENT.ON_SHELL_DAEMON) {
        return;
      }
      const payload = item.context as RespAgentShellDaemon;
      if (payload.chatId !== chatId) {
        return;
      }
      applyDaemons(payload.daemons);
      openNewTtyTerminals(payload.daemons);
    });
  }, [agentService, chatId]);

  if (daemons.length === 0) {
    return null;
  }

  return (
    <Box className="tm-agent-daemon">
      <ButtonBase disableRipple className="tm-agent-daemon-head" onClick={handleToggle}>
        {expanded ? <KeyboardArrowDown className="tm-agent-daemon-chevron" /> : <KeyboardArrowRight className="tm-agent-daemon-chevron" />}
        <Box className="tm-agent-daemon-title">{resolveHeader()}</Box>
      </ButtonBase>
      {expanded ? (
        <Box className="tm-agent-daemon-list">
          {daemons.map(row => (
            <Fragment key={row.processId}>
              <Box className="tm-agent-daemon-row">
                {canExpand(row) ? (
                  <ButtonBase disableRipple className="tm-agent-daemon-fold" onClick={() => handleToggleRow(row)}>
                    {isOpen(row) ? <KeyboardArrowDown className="tm-agent-daemon-chevron" /> : <KeyboardArrowRight className="tm-agent-daemon-chevron" />}
                  </ButtonBase>
                ) : null}
                {row.shellId ? <Terminal className="tm-agent-daemon-icon" /> : <Loop className="tm-agent-daemon-icon" />}
                <Box className="tm-agent-daemon-cmd" title={row.command}>
                  {resolveTitle(row)}
                </Box>
                {row.tty ? (
                  <ButtonBase disableRipple className="tm-agent-daemon-action" onClick={() => handleOpenTerminal(row)}>
                    {t('agent.daemon.openTerminal')}
                  </ButtonBase>
                ) : null}
                <ButtonBase disableRipple className="tm-agent-daemon-action" disabled={stoppingIds.includes(row.processId)} onClick={() => handleStop(row)}>
                  {t('agent.daemon.stop')}
                </ButtonBase>
              </Box>
              {isOpen(row) ? (
                <Box className="tm-agent-daemon-log">
                  <AgentShellResult content={row.output} running={true} active={true} />
                </Box>
              ) : null}
            </Fragment>
          ))}
        </Box>
      ) : null}
    </Box>
  );
}
