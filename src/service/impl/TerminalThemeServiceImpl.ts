import {inject, injectable} from 'inversify';
import {BaseServiceImpl} from './BaseServiceImpl.ts';
import type {ConfigService} from '../ConfigService.ts';
import {ConfigServiceImpl} from './ConfigServiceImpl.ts';
import {TerminalThemeService} from '../TerminalThemeService.ts';
import type {TabConfig} from '@/common/components/tab/core/types.ts';
import {APP_THEME, APP_THEME_OPTIONS} from '@/common/model/AppTheme.ts';
import {resolveContrastForeground} from '@/page/common/util/colorContrastUtil.ts';
import type {TabManagerService} from '@/common/components/tab/service/TabManagerService.ts';
import {DEFAULT_TERMINAL_THEME_ID, TERMINAL_THEME_OPTIONS} from '@/page/common/util/terminal.ts';
import {TabManagerServiceImpl} from '@/common/components/tab/service/impl/TabManagerServiceImpl.ts';
import {EVENT_TYPE, TAB_LAYOUT_REASON, TabLayoutEventData} from '@/common/components/tab/core/types.ts';

@injectable()
export class TerminalThemeServiceImpl extends BaseServiceImpl implements TerminalThemeService {
    private readonly ATTR_APP_THEME = 'data-app-theme';
    private readonly listeners = new Set<() => void>();
    private readonly paneThemes = new Map<string, string>();
    private readonly stickyPaneIdByTab = new Map<string, string>();
    private appThemeId: string = APP_THEME.DEFAULT;

    constructor(
        @inject(TabManagerServiceImpl) private readonly tabManagerService: TabManagerService,
        @inject(ConfigServiceImpl) private readonly configService: ConfigService,
    ) {
        super();
    }

    async start(): Promise<void> {
        this.appThemeId = this.configService.selectAppThemeId();
        this.tabManagerService.addEventListener((event: number, data: any) => {
            if (event === EVENT_TYPE.ACTIVE) {
                this.recompute();
            } else if (event === EVENT_TYPE.LAYOUT) {
                const reason = (data as TabLayoutEventData)?.reason;
                if (reason === TAB_LAYOUT_REASON.FOCUS || reason === TAB_LAYOUT_REASON.PANE_MUTATION) {
                    this.recompute();
                }
            }
        });
        this.recompute();
        return super.start();
    }

    addChangeListener(listener: () => void): () => void {
        this.listeners.add(listener);
        return () => {
            this.listeners.delete(listener);
        };
    }

    registerPaneTheme(paneId: string, themeId: string): void {
        this.paneThemes.set(paneId, themeId);
        this.recompute();
    }

    /* pane 的渲染主题取自它所在 tab 的激活主题:合并时同 tab 的所有 pane(含 sftp)统一跟随
       激活 pane;拖出去独立成 tab 后该 tab 只剩自己,于是自然还原成自己的主题。 */
    getPaneThemeId(paneId: string): string {
        const tab = this.tabManagerService.getTabs().find(item => item.panes.some(pane => pane.id === paneId));
        const tabThemeId = tab?.id ? this.resolveTabThemeId(tab.id, tab) : undefined;
        return tabThemeId ?? this.paneThemes.get(paneId) ?? DEFAULT_TERMINAL_THEME_ID;
    }

    getAppThemeId(): string {
        return this.appThemeId;
    }

    setAppThemeId(themeId: string): void {
        if (!APP_THEME_OPTIONS.some(theme => theme.id === themeId)) {
            throw new Error(`app theme invalid: ${themeId}`);
        }
        this.appThemeId = themeId;
        this.configService.saveAppThemeId(themeId);
        this.recompute();
    }

    private recompute(): void {
        this.pruneClosedPanes();
        const tabId = this.tabManagerService.getActiveTabId();
        const tab = tabId ? this.tabManagerService.getTab(tabId) : null;
        if (!tabId || !tab) {
            this.applyTheme(this.appThemeId);
            return;
        }
        this.syncStickyPaneId(tabId, tab);
        this.applyTheme(this.appThemeId, this.resolveTabThemeId(tabId, tab));
    }

    private applyTheme(themeId: string, terminalThemeId?: string): void {
        const root = document.documentElement;
        const terminalTheme = terminalThemeId ? TERMINAL_THEME_OPTIONS.find(theme => theme.id === terminalThemeId) : undefined;
        if (terminalThemeId && !terminalTheme) {
            throw new Error(`terminal theme invalid: ${terminalThemeId}`);
        }
        const light = terminalTheme ? resolveContrastForeground(terminalTheme.bg) === '#000000' : themeId === APP_THEME.LIGHT;
        root.classList.toggle('dark', !light);
        root.style.colorScheme = light ? 'light' : 'dark';
        if (light) {
            root.setAttribute(this.ATTR_APP_THEME, APP_THEME.LIGHT);
        } else {
            root.removeAttribute(this.ATTR_APP_THEME);
        }
        if (terminalTheme) {
            root.setAttribute('data-terminal-theme', terminalTheme.id);
        } else {
            root.removeAttribute('data-terminal-theme');
        }
        for (const listener of this.listeners) {
            listener();
        }
    }

    private pruneClosedPanes(): void {
        const alivePaneIds = new Set<string>();
        for (const tab of this.tabManagerService.getTabs()) {
            for (const pane of tab.panes) {
                if (pane.id) {
                    alivePaneIds.add(pane.id);
                }
            }
        }
        for (const paneId of this.paneThemes.keys()) {
            if (!alivePaneIds.has(paneId)) {
                this.paneThemes.delete(paneId);
            }
        }
        for (const [tabId, stickyPaneId] of this.stickyPaneIdByTab) {
            if (!alivePaneIds.has(stickyPaneId)) {
                this.stickyPaneIdByTab.delete(tabId);
            }
        }
    }

    /* sftp 之类的 pane 不注册主题,聚焦到它时沿用同 tab 上一个终端 pane 的主题,避免整块 chrome 跳回默认色。 */
    private resolveTabThemeId(tabId: string, tab: TabConfig): string | undefined {
        const focusedPaneId = this.resolveFocusedPaneId(tab);
        const focusedThemeId = focusedPaneId ? this.paneThemes.get(focusedPaneId) : undefined;
        if (focusedThemeId) {
            return focusedThemeId;
        }
        const stickyPaneId = this.stickyPaneIdByTab.get(tabId);
        if (!stickyPaneId || !tab.panes.some(pane => pane.id === stickyPaneId)) {
            return undefined;
        }
        return this.paneThemes.get(stickyPaneId);
    }

    private syncStickyPaneId(tabId: string, tab: TabConfig): void {
        const focusedPaneId = this.resolveFocusedPaneId(tab);
        if (focusedPaneId && this.paneThemes.has(focusedPaneId)) {
            this.stickyPaneIdByTab.set(tabId, focusedPaneId);
            return;
        }
        const stickyPaneId = this.stickyPaneIdByTab.get(tabId);
        if (stickyPaneId && !tab.panes.some(pane => pane.id === stickyPaneId)) {
            this.stickyPaneIdByTab.delete(tabId);
        }
    }

    /* focusedPaneId 可能指向已被拖走的 pane,必须校验它还在本 tab 内,否则分离后老 tab 会继续用走掉那个 pane 的主题。 */
    private resolveFocusedPaneId(tab: TabConfig): string | undefined {
        const focusedPaneId = tab.focusedPaneId;
        if (focusedPaneId && tab.panes.some(pane => pane.id === focusedPaneId)) {
            return focusedPaneId;
        }
        return tab.panes[0]?.id;
    }

}
