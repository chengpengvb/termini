import { Box } from '@mui/material';
import { Icon } from '@iconify/react';
import { useI18n } from '@/common/i18n/useI18n.ts';
import iconBroadcast from '@/assets/icon/broadcast';
import { useCallback, useEffect, useState } from 'react';
import { EVENT_TYPE } from '@/common/components/tab/core/types';
import { useTabManagerService, useTerminalBroadcastService } from '@/common/context/useAppHooks.tsx';

export interface TerminalBroadcastProps {
  paneId: string;
  registerKeyHandler: (handler: (event: KeyboardEvent) => boolean) => () => void;
}

const SHORTCUT_KEYS = ['Ctrl', 'Shift', 'B'];

export function TerminalBroadcast(props: TerminalBroadcastProps) {
  const { t } = useI18n();
  const { paneId, registerKeyHandler } = props;
  const tabManagerService = useTabManagerService();
  const terminalBroadcastService = useTerminalBroadcastService();
  const [active, setActive] = useState<boolean>(() => terminalBroadcastService.isTarget(paneId));

  const toggleBroadcast = useCallback((): void => {
    const tab = tabManagerService.getTabs().find(item => item.panes.some(pane => pane.id === paneId));
    if (!tab) {
      return;
    }
    if (!tab.id) {
      throw new Error('tab.id unavailable');
    }
    if (terminalBroadcastService.isTarget(paneId)) {
      terminalBroadcastService.setTargets(tab.id, false);
      return;
    }
    terminalBroadcastService.setTargets(tab.id, true);
  }, [paneId, tabManagerService, terminalBroadcastService]);

  useEffect(() => {
    const syncActive = (): void => {
      setActive(terminalBroadcastService.isTarget(paneId));
    };
    const offBroadcast = terminalBroadcastService.addChangeListener(syncActive);
    const offTab = tabManagerService.addEventListener((event: number) => {
      if (event !== EVENT_TYPE.OPEN && event !== EVENT_TYPE.CLOSE && event !== EVENT_TYPE.LAYOUT) {
        return;
      }
      syncActive();
    });
    return () => {
      offBroadcast();
      offTab();
    };
  }, [paneId, tabManagerService, terminalBroadcastService]);

  useEffect(() => {
    return registerKeyHandler((event: KeyboardEvent): boolean => {
      if (event.type !== 'keydown') {
        return true;
      }
      const isBroadcast = (event.ctrlKey || event.metaKey) && event.shiftKey && event.key.toLowerCase() === 'b';
      if (!isBroadcast) {
        return true;
      }
      toggleBroadcast();
      return false;
    });
  }, [registerKeyHandler, toggleBroadcast]);

  useEffect(() => {
    const toolId = tabManagerService.addPaneTool({
      paneId,
      tabBar: {
        keys: SHORTCUT_KEYS,
        onClick: toggleBroadcast,
        icon: <Icon icon={iconBroadcast} />,
        title: t('protocol.terminal.broadcastLabel'),
        active: () => terminalBroadcastService.isTarget(paneId),
      },
    });
    return () => {
      tabManagerService.removePaneTool(toolId);
    };
  }, [t, paneId, tabManagerService, terminalBroadcastService, toggleBroadcast]);

  if (!active) {
    return null;
  }

  return <Box className="tm-terminal-broadcast-frame" />;
}
