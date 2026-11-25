import {Box, ButtonBase} from '@mui/material';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {useEffect, useRef, useState} from 'react';
import {generateId} from '@/common/util/IdUtil.ts';
import {isEmpty} from '@/common/util/StringUtil.ts';
import type {AiChat} from '@/common/model/AiChat.ts';
import type {AiSkill} from '@/common/model/AiSkill.ts';
import {LeftMenu} from '@/page/agent/left/LeftMenu.tsx';
import {McpManage} from '@/page/agent/mcp/McpManage.tsx';
import {AGENT_MENU} from '@/page/common/model/AgentMenu.ts';
import {PlanManage} from '@/page/agent/plan/PlanManage.tsx';
import {SkillManage} from '@/page/agent/skill/SkillManage.tsx';
import {ArrowBack, Menu as MenuIcon} from '@mui/icons-material';
import {useAiChatService} from '@/common/context/useAppHooks.tsx';
import {CHAT_TARGET_TYPE} from '@/common/const/ChatTargetConst.ts';
import {MobileHeader} from '@/page/home/components/MobileHeader.tsx';
import {NewChatIcon} from '@/assets/icon/components/NewChatIcon.tsx';
import {useBackDismiss} from '@/page/common/hooks/useBackDismiss.ts';
import {usePlatformMobile} from '@/common/context/usePlatformMobile';
import {AgentSplitDivider} from '@/page/agent/common/AgentSplitDivider.tsx';
import {MainBody, type MainBodyHandle} from '@/page/agent/main/MainBody.tsx';
import {registerGuidanceAgentNewChat} from '@/page/common/util/guidanceSurface.ts';

export function AgentPage() {
    const {t} = useI18n();
    const mobileViewport = usePlatformMobile();
    const aiChatService = useAiChatService();
    const rootRef = useRef<HTMLDivElement>(null);
    const [leftWidth, setLeftWidth] = useState(260);
    const [sessionId] = useState(() => generateId());
    const leftMenuRef = useRef<HTMLDivElement>(null);
    const mainBodyRef = useRef<MainBodyHandle>(null);
    const [drawerOpen, setDrawerOpen] = useState(false);
    const [chatList, setChatList] = useState<AiChat[]>([]);
    const [chatId, setChatId] = useState<string | null>(null);
    const [planChatId, setPlanChatId] = useState<string | null>(null);
    const [currentMenu, setCurrentMenu] = useState<number>(AGENT_MENU.NEW);
    const closeDrawer = (): void => {
        setDrawerOpen(false);
    };


    function isChatMenu(): boolean {
        if (currentMenu === AGENT_MENU.NEW) {
            return true;
        }
        if (currentMenu === AGENT_MENU.HISTORY) {
            return true;
        }
        return false;
    }

    function isPlanChatOpen(): boolean {
        return currentMenu === AGENT_MENU.PLAN && planChatId !== null;
    }

    function isChatVisible(): boolean {
        return isChatMenu() || isPlanChatOpen();
    }

    function resolveMobileTitle(): string {
        if (currentMenu === AGENT_MENU.SKILL) {
            return t('agent.menu.skills');
        }
        if (currentMenu === AGENT_MENU.MCP) {
            return t('agent.menu.mcp');
        }
        if (currentMenu === AGENT_MENU.PLAN && planChatId === null) {
            return t('agent.menu.plan');
        }
        const current = chatList.find(chat => chat.id === chatId);
        if (current) {
            return current.title;
        }
        return t('agent.menu.newChat');
    }

    function handleMenuChange(menu: number): void {
        setCurrentMenu(menu);
        setPlanChatId(null);
        if (menu === AGENT_MENU.NEW) {
            setChatId(null);
        }
        if (mobileViewport) {
            setDrawerOpen(false);
        }
    }

    function handleSelectChat(nextChatId: string | null): void {
        setPlanChatId(null);
        setCurrentMenu(nextChatId ? AGENT_MENU.HISTORY : AGENT_MENU.NEW);
        setChatId(nextChatId);
        if (mobileViewport) {
            setDrawerOpen(false);
        }
    }

    function handleOpenPlanChat(nextChatId: string): void {
        setPlanChatId(nextChatId);
        setChatId(nextChatId);
        if (mobileViewport) {
            setDrawerOpen(false);
        }
    }

    function handleBackToRuns(): void {
        setPlanChatId(null);
        setChatId(null);
    }

    function handleUseSkill(skill: AiSkill): void {
        if (isEmpty(skill.id)) {
            return;
        }
        if (isEmpty(skill.name)) {
            return;
        }
        setCurrentMenu(chatId ? AGENT_MENU.HISTORY : AGENT_MENU.NEW);
        if (mobileViewport) {
            setDrawerOpen(false);
        }
        mainBodyRef.current?.applyReference({
            data: null,
            id: skill.id,
            title: skill.name,
            kind: CHAT_TARGET_TYPE.SKILL,
        });
    }

    function renderMobileHeader() {
        return (
            <MobileHeader
                title={resolveMobileTitle()}
                leading={(
                    <ButtonBase disableRipple className="tm-mobile-header-icon-btn" onClick={() => setDrawerOpen(true)} aria-label={t('agent.mobile.openMenu')}>
                        <MenuIcon/>
                    </ButtonBase>
                )}
                action={isChatVisible() ? (
                    <ButtonBase disableRipple className="tm-mobile-header-icon-btn" onClick={() => handleMenuChange(AGENT_MENU.NEW)} aria-label={t('agent.menu.newChat')}>
                        <NewChatIcon fontSize={22}/>
                    </ButtonBase>
                ) : null}
            />
        );
    }

    useBackDismiss(mobileViewport && drawerOpen, closeDrawer, 'agent-drawer');

    useEffect(() => {
        return registerGuidanceAgentNewChat(() => {
            setCurrentMenu(AGENT_MENU.NEW);
            setChatId(null);
            setDrawerOpen(false);
        });
    }, []);

    useEffect(() => {
        if (!mobileViewport) {
            return;
        }
        aiChatService.selectAll().then(setChatList);
        return aiChatService.addAiChatListChangeListener(setChatList);
    }, [aiChatService, mobileViewport]);

    return (
        <Box ref={rootRef} className="tm-agent-root">
            {mobileViewport ? (
                <Box className={drawerOpen ? 'tm-agent-drawer-root is-open' : 'tm-agent-drawer-root'}>
                    <Box className="tm-agent-drawer-backdrop" onClick={() => setDrawerOpen(false)}/>
                    <Box className="tm-agent-drawer-panel">
                        <LeftMenu width={280} currentMenu={currentMenu} chatId={chatId} drawerOpen={drawerOpen} onMenuChange={handleMenuChange} setChatId={handleSelectChat}/>
                    </Box>
                </Box>
            ) : (
                <>
                    <LeftMenu rootRef={leftMenuRef} width={leftWidth} currentMenu={currentMenu} chatId={chatId} onMenuChange={handleMenuChange} setChatId={handleSelectChat}/>
                    <AgentSplitDivider containerRef={rootRef} widthTargetRef={leftMenuRef} onWidthChange={setLeftWidth}/>
                </>
            )}
            <Box className="tm-agent-main-shell">
                {mobileViewport ? renderMobileHeader() : null}
                {isPlanChatOpen() ? (
                    <ButtonBase disableRipple className="tm-agent-plan-back" onClick={handleBackToRuns}>
                        <ArrowBack sx={{fontSize: 15}}/>
                        <span>{t('agent.plan.backToRuns')}</span>
                    </ButtonBase>
                ) : null}
                <Box className={isChatVisible() ? 'tm-agent-main-host' : 'tm-agent-main-host is-concealed'}>
                    <MainBody ref={mainBodyRef} sessionId={sessionId} chatId={chatId} setChatId={handleSelectChat}/>
                </Box>
                {currentMenu === AGENT_MENU.SKILL ? (
                    <SkillManage onUseSkill={handleUseSkill}/>
                ) : null}
                {currentMenu === AGENT_MENU.MCP ? (
                    <McpManage/>
                ) : null}
                {currentMenu === AGENT_MENU.PLAN ? (
                    <Box className={planChatId === null ? 'tm-agent-plan-host' : 'tm-agent-plan-host is-concealed'}>
                        <PlanManage onOpenChat={handleOpenPlanChat}/>
                    </Box>
                ) : null}
            </Box>
        </Box>
    );
}
