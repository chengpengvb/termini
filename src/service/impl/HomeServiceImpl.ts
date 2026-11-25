import {createElement} from 'react';
import {Add} from '@mui/icons-material';
import {inject, injectable} from 'inversify';
import {HomeService} from "../HomeService.ts";
import {translate} from '@/common/i18n/i18n.ts';
import {HomePage} from "@/page/home/HomePage.tsx";
import {AgentPage} from '@/page/agent/AgentPage.tsx';
import {BaseServiceImpl} from "./BaseServiceImpl.ts";
import {ShellServiceImpl} from "./ShellServiceImpl.ts";
import {ConfigServiceImpl} from "./ConfigServiceImpl.ts";
import {NewTabTool} from '@/page/home/history/NewTabTool.tsx';
import {ShellPage} from '@/page/protocol/shell/ShellPage.tsx';
import {DeckIcon} from '@/assets/icon/components/DeckIcon.tsx';
import {HomeIcon} from '@/assets/icon/components/HomeIcon.tsx';
import {HistoryPage} from '@/page/home/history/HistoryPage.tsx';
import {WebSocketServiceImpl} from '@/service/impl/WebSocketServiceImpl.ts';
import type {RespSocketContext} from '@/common/model/resp/RespWsContext.ts';
import {BAR_TOOL_SLOT, EVENT_TYPE} from '@/common/components/tab/core/types.ts';
import type {TabManagerService} from "@/common/components/tab/service/TabManagerService.ts";
import {guidanceAgentNewChat, registerGuidanceTabs} from '@/page/common/util/guidanceSurface.ts';
import {TabManagerServiceImpl} from "@/common/components/tab/service/impl/TabManagerServiceImpl.ts";
import {resolveShellIcon, resolveShellKind, resolveShellLabel} from '@/page/common/util/shellUtil.ts';
import {RESP_EVENT_WINDOW, TRAY_ACTION, type OpenHerePayload} from '@/common/model/resp/event/RespEventWindow.ts';

@injectable()
export class HomeServiceImpl extends BaseServiceImpl implements HomeService {
    private readonly TAB_VALUE_HOME = 'Home';
    private readonly TAB_VALUE_DECK = 'Deck';
    private persistenceInited: boolean = false;
    private openHereReady: boolean = false;
    private lastOpenHereSeq: number = 0;
    private readonly TAB_ID_DECK = 'tm-tab-deck';
    private readonly TAB_ID_HOME = 'tm-tab-home';
    private readonly pendingOpenHere: OpenHerePayload[] = [];
    private readonly STORAGE_KEY_SELECTED_TOP_TAB = 'argus:tab:selected-home-deck';

    constructor(
        @inject(TabManagerServiceImpl) private readonly tabManagerService: TabManagerService,
        @inject(WebSocketServiceImpl) private readonly webSocketService: WebSocketServiceImpl,
        @inject(ConfigServiceImpl) private readonly configService: ConfigServiceImpl,
        @inject(ShellServiceImpl) private readonly shellService: ShellServiceImpl,
    ) {
        super();
        this.webSocketService.addWsListener((context: RespSocketContext) => this.onOpenHereEvent(context));
    }

    async start(): Promise<void> {
        registerGuidanceTabs({
            showHomeTab: () => {
                this.tabManagerService.setActiveTabId(this.TAB_ID_HOME);
            },
            showDeckTab: () => {
                this.tabManagerService.setActiveTabId(this.TAB_ID_DECK);
            },
        });
        this.installTrayActionListener();
        void this.initAfterTabReady();
        return super.start();
    }

    private onOpenHereEvent(context: RespSocketContext): void {
        if (context.type !== RESP_EVENT_WINDOW.ON_OPEN_HERE) {
            return;
        }
        const payload = context.payload as OpenHerePayload;
        if (payload.seq <= this.lastOpenHereSeq) {
            return;
        }
        if (!this.openHereReady) {
            this.pendingOpenHere.push(payload);
            return;
        }
        this.lastOpenHereSeq = payload.seq;
        void this.openHereTab(payload.path);
    }

    private flushOpenHere(): void {
        const queued = this.pendingOpenHere.splice(0);
        for (const item of queued) {
            if (item.seq <= this.lastOpenHereSeq) {
                continue;
            }
            this.lastOpenHereSeq = item.seq;
            void this.openHereTab(item.path);
        }
    }

    private async openHereTab(cwd: string): Promise<void> {
        const config = await this.configService.selectConfig();
        if (!config) {
            console.warn('[home] default config is not found');
            return;
        }
        if (!config.local_shell_id) {
            console.warn('[home] local_shell_id is not found');
            return;
        }
        const shells = await this.shellService.discover();
        const shell = shells.find(item => item.shell_id === config.local_shell_id);
        if (!shell) {
            console.warn(`[home] shell not found: ${config.local_shell_id}`);
            return;
        }
        const kind = resolveShellKind(shell.shell_id);
        this.tabManagerService.openTab({
            panes: [{
                tabBar: {
                    canClose: true,
                    title: resolveShellLabel(kind),
                    icon: resolveShellIcon(kind),
                },
                content: createElement(ShellPage, { shellDescr: shell, cwd }),
            }],
        });
    }

    private installTrayActionListener(): void {
        this.webSocketService.addWsListener((context: RespSocketContext) => {
            if (context.type !== RESP_EVENT_WINDOW.ON_TRAY_ACTION) {
                return;
            }
            const action = Number(context.payload);
            if (action === TRAY_ACTION.HOME) {
                this.tabManagerService.setActiveTabId(this.TAB_ID_HOME);
            } else if (action === TRAY_ACTION.NEW_CHAT) {
                this.tabManagerService.setActiveTabId(this.TAB_ID_DECK);
                guidanceAgentNewChat();
            }
        });
    }

    private async initAfterTabReady(): Promise<void> {
        await this.initHomePage();
        await this.initDeckPage();
        this.installTopTabPersistence();
        this.restoreTopTabActive();
        this.openHereReady = true;
        this.flushOpenHere();
    }

    private async initHomePage() {
        await this.tabManagerService.waitReady();
        this.tabManagerService.openTab({
            id: this.TAB_ID_HOME,
            panes: [{
                tabBar: {
                    title: translate('tab.home'),
                    canClose: false,
                    canMerge: false,
                    icon: createElement(HomeIcon, {fontSize: 18, className: 'tm-tab-home-icon'}),
                },
                content: createElement(HomePage),
            }
            ],
        });
        this.tabManagerService.addBarTool({
            tabBar: {
                title: translate('tab.newPage'),
                slot: BAR_TOOL_SLOT.STRIP,
                icon: createElement(NewTabTool),
                onClick: () => {
                    this.tabManagerService.openTab({
                        panes: [{
                            tabBar: {
                                canClose: true,
                                title: translate('tab.newTab'),
                                icon: createElement(Add, {fontSize: 'small'}),
                            },
                            content: createElement(HistoryPage),
                        }],
                    });
                },
            },
        });
    }

    private async initDeckPage() {
        await this.tabManagerService.waitReady();
        this.tabManagerService.openTab({
            id: this.TAB_ID_DECK,
            panes: [{
                tabBar: {
                    canClose: false,
                    canMerge: false,
                    title: translate('tab.deck'),
                    icon: createElement(DeckIcon, {fontSize: 18, className: 'tm-tab-deck-icon'}),
                },
                content: createElement(AgentPage),
            }],
        });
    }

    private installTopTabPersistence(): void {
        if (this.persistenceInited) {
            return;
        }
        this.persistenceInited = true;
        this.tabManagerService.addEventListener((event: number, data: any) => {
            if (event !== EVENT_TYPE.ACTIVE) {
                return;
            }
            const tabId = data as string | undefined;
            if (tabId === this.TAB_ID_HOME) {
                window.localStorage.setItem(this.STORAGE_KEY_SELECTED_TOP_TAB, this.TAB_VALUE_HOME);
            } else if (tabId === this.TAB_ID_DECK) {
                window.localStorage.setItem(this.STORAGE_KEY_SELECTED_TOP_TAB, this.TAB_VALUE_DECK);
            }
        });
    }

    private restoreTopTabActive(): void {
        const raw = window.localStorage.getItem(this.STORAGE_KEY_SELECTED_TOP_TAB);
        if (raw === this.TAB_VALUE_DECK) {
            this.tabManagerService.setActiveTabId(this.TAB_ID_DECK);
        } else {
            this.tabManagerService.setActiveTabId(this.TAB_ID_HOME);
        }
    }

}

