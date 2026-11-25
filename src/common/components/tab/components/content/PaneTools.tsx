import { useEffect, useState } from 'react';
import { Box, ButtonBase, Tooltip } from '@mui/material';
import { useTabContext } from '../../hooks/useTabContext';
import { EVENT_TYPE, type PaneToolConfig } from '../../core/types';
import { usePlatformMobile } from '@/common/context/usePlatformMobile';
import { useTerminalBroadcastService } from '@/common/context/useAppHooks.tsx';
import type { MouseEvent as ReactMouseEvent, PointerEvent as ReactPointerEvent } from 'react';

interface PaneToolsProps {
    paneId: string;
}

function resolvePaneToolActive(tool: PaneToolConfig): boolean {
    if (!tool.tabBar.active) {
        return false;
    }
    return tool.tabBar.active();
}

export function PaneTools({ paneId }: PaneToolsProps) {
    const mobile = usePlatformMobile();
    const { manager } = useTabContext();
    const terminalBroadcastService = useTerminalBroadcastService();
    const [tools, setTools] = useState<PaneToolConfig[]>(() => manager.getPaneTools(paneId));

    const handlePointerDown = (e: ReactPointerEvent<HTMLButtonElement>): void => {
        e.stopPropagation();
    };

    const handleClick = (e: ReactMouseEvent<HTMLButtonElement>, tool: PaneToolConfig): void => {
        e.preventDefault();
        e.stopPropagation();
        tool.tabBar.onClick();
    };

    useEffect(() => {
        const refresh = (): void => {
            setTools(manager.getPaneTools(paneId));
        };
        const offManager = manager.addEventListener(event => {
            if (event !== EVENT_TYPE.PANE_TOOL && event !== EVENT_TYPE.OPEN && event !== EVENT_TYPE.CLOSE && event !== EVENT_TYPE.LAYOUT) {
                return;
            }
            refresh();
        });
        const offBroadcast = terminalBroadcastService.addChangeListener(refresh);
        return () => {
            offManager();
            offBroadcast();
        };
    }, [manager, paneId, terminalBroadcastService]);

    if (tools.length === 0) {
        return null;
    }

    return (
        <Box className="tm-pane-tools">
            {tools.map(tool => {
                const btn = (
                    <ButtonBase key={tool.id} aria-label={tool.tabBar.title} onPointerDown={handlePointerDown} onClick={e => handleClick(e, tool)} className={resolvePaneToolActive(tool) ? 'tm-pane-tool is-active' : 'tm-pane-tool'}>
                        {tool.tabBar.icon}
                    </ButtonBase>
                );
                if (mobile) {
                    return btn;
                }
                const tip = (
                    <Box className="tm-pane-tool-tip">
                        <Box component="span" className="tm-pane-tool-tip-label">{tool.tabBar.title}</Box>
                        {tool.tabBar.keys ? tool.tabBar.keys.map(key => (
                            <Box key={key} component="span" className="tm-pane-tool-tip-key">{key}</Box>
                        )) : null}
                    </Box>
                );
                return (
                    <Tooltip key={tool.id} title={tip} placement="left" disableInteractive disableTouchListener slotProps={{tooltip: {className: 'tm-pane-tool-pop'}}}>
                        {btn}
                    </Tooltip>
                );
            })}
        </Box>
    );
}
