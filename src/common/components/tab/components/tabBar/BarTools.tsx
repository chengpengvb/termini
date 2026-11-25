import {useEffect, useState} from 'react';
import {Box, ButtonBase} from '@mui/material';
import {useTabContext} from '../../hooks/useTabContext';
import {BAR_TOOL_SLOT, type BarToolConfig, EVENT_TYPE} from '../../core/types';

interface BarToolsProps {
    slot: number;
}

export function BarTools({slot}: BarToolsProps) {
    const {manager} = useTabContext();
    const [tools, setTools] = useState<BarToolConfig[]>(() => manager.getBarTools());

    useEffect(() => {
        return manager.addEventListener((event, data) => {
            if (event !== EVENT_TYPE.BAR_TOOL) {
                return;
            }
            setTools([...(data as BarToolConfig[])]);
        });
    }, [manager]);

    const slotTools = tools.filter(tool => (tool.tabBar.slot ?? BAR_TOOL_SLOT.RIGHT) === slot);
    if (slotTools.length === 0) {
        return null;
    }

    return (
        <Box className={slot === BAR_TOOL_SLOT.STRIP ? 'tm-tab-bar-tools tm-tab-bar-tools-strip' : 'tm-tab-bar-tools'}>
            {slotTools.map(tool => (
                <ButtonBase key={tool.id} aria-label={tool.tabBar.title} className="tm-btn-highlight tm-bar-tool-btn" data-tm-bar-avatar={slot === BAR_TOOL_SLOT.STRIP ? undefined : 'true'} onClick={event => {
                    const target = event.nativeEvent.target;
                    if (!(target instanceof Node) || !event.currentTarget.contains(target)) {
                        return;
                    }
                    tool.tabBar.onClick?.();
                }}>
                    {tool.tabBar.icon}
                </ButtonBase>
            ))}
        </Box>
    );
}
