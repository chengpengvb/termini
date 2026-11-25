import {Box} from '@mui/material';
import type {RefObject} from 'react';
import {AGENT_MENU} from '@/page/common/model/AgentMenu.ts';
import {AgentMenu} from '@/page/agent/left/menu/AgentMenu.tsx';
import {DeviceTree} from '@/page/agent/left/device/DeviceTree.tsx';
import {ChatHistory} from '@/page/agent/left/session/ChatHistory.tsx';

interface LeftMenuProps {
    width: number;
    currentMenu: number;
    chatId: string | null;
    drawerOpen?: boolean;
    onMenuChange: (menu: number) => void;
    setChatId: (chatId: string | null) => void;
    rootRef?: RefObject<HTMLDivElement | null>;
}

export function LeftMenu(props: LeftMenuProps) {
    const {width, currentMenu, chatId, drawerOpen = true, onMenuChange, setChatId, rootRef} = props;

    return (
        <Box ref={rootRef} className="tm-agent-left" style={{width}}>
            <AgentMenu currentMenu={currentMenu} onMenuChange={onMenuChange}/>
            <ChatHistory chatId={chatId} selected={currentMenu === AGENT_MENU.HISTORY} drawerOpen={drawerOpen} setChatId={setChatId}/>
            <DeviceTree selected={currentMenu === AGENT_MENU.DEVICE}/>
        </Box>
    );
}
