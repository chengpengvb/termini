import {Box} from '@mui/material';
import {useI18n} from '@/common/i18n/useI18n.ts';
import HubOutlined from '@mui/icons-material/HubOutlined';
import {AGENT_MENU} from '@/page/common/model/AgentMenu.ts';
import {SkillIcon} from '@/assets/icon/components/SkillIcon.tsx';
import ScheduleOutlined from '@mui/icons-material/ScheduleOutlined';
import {NewChatIcon} from '@/assets/icon/components/NewChatIcon.tsx';
import {NavMenuItem} from '@/page/common/components/nav/NavMenuItem.tsx';

interface AgentMenuProps {
    currentMenu: number;
    onMenuChange: (menu: number) => void;
}

export function AgentMenu(props: AgentMenuProps) {
    const {t} = useI18n();
    const {currentMenu, onMenuChange} = props;

    const handleSelectNew = (): void => {
        onMenuChange(AGENT_MENU.NEW);
    };

    const handleSelectSkill = (): void => {
        onMenuChange(AGENT_MENU.SKILL);
    };

    const handleSelectMcp = (): void => {
        onMenuChange(AGENT_MENU.MCP);
    };

    const handleSelectPlan = (): void => {
        onMenuChange(AGENT_MENU.PLAN);
    };

    return (
        <Box className="tm-agent-nav-menu tm-nav-menu">
            <NavMenuItem label={t('agent.menu.newChat')} selected={currentMenu === AGENT_MENU.NEW} icon={<NewChatIcon className="tm-nav-item-icon"/>} onClick={handleSelectNew}/>
            <NavMenuItem label={t('agent.menu.skills')} selected={currentMenu === AGENT_MENU.SKILL} icon={<SkillIcon className="tm-nav-item-icon"/>} onClick={handleSelectSkill}/>
            <NavMenuItem label={t('agent.menu.mcp')} selected={currentMenu === AGENT_MENU.MCP} icon={<HubOutlined className="tm-nav-item-icon"/>} onClick={handleSelectMcp}/>
            <NavMenuItem label={t('agent.menu.plan')} selected={currentMenu === AGENT_MENU.PLAN} icon={<ScheduleOutlined className="tm-nav-item-icon"/>} onClick={handleSelectPlan}/>
        </Box>
    );
}
