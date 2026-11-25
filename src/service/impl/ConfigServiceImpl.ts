import {inject, injectable} from 'inversify';
import type {ConfigService} from '../ConfigService';
import {BaseApiServiceImpl} from './BaseApiServiceImpl';
import {DEFAULT_CONFIG_ID} from '@/common/model/Config';
import type {Config, ModelRole} from '@/common/model/Config';
import {ENTITY_TYPE} from '@/common/const/EntityTypeConst.ts';
import type {SaveNotifyReq} from '@/common/model/req/ReqConfig.ts';
import {APP_THEME, APP_THEME_OPTIONS} from '@/common/model/AppTheme.ts';
import {WebSocketServiceImpl} from '@/service/impl/WebSocketServiceImpl.ts';
import {DEFAULT_TERMINAL_CURSOR_STYLE, DEFAULT_TERMINAL_FONT_ID, DEFAULT_TERMINAL_FONT_SIZE, DEFAULT_TERMINAL_THEME_ID, normalizeTerminalCursorStyle, normalizeTerminalFontSizeRem, TERMINAL_THEME_OPTIONS, type TerminalSetting} from '@/page/common/util/terminal.ts';

@injectable()
export class ConfigServiceImpl extends BaseApiServiceImpl<Config> implements ConfigService {

    private readonly BACKGROUND_HINT_DONE_VALUE = '1';
    private readonly APP_THEME_STORAGE_KEY = 'argus:app:theme';
    private readonly BACKGROUND_HINT_STORAGE_KEY = 'argus:session:backgroundHint:done';
    private readonly TERMINAL_SETTING_STORAGE_PREFIX = 'argus:terminal:setting:';

    constructor(
        @inject(WebSocketServiceImpl) webSocketService: WebSocketServiceImpl,
    ) {
        super(webSocketService);
    }

    protected getNotifyEntityType(): number {
        return ENTITY_TYPE.CONFIG;
    }

    protected getPrefix(): string {
        return '/config';
    }

    async selectConfig(): Promise<Config | null> {
        return this.selectById(DEFAULT_CONFIG_ID);
    }

    selectTerminalSetting(settingKey: string): TerminalSetting {
        const raw = window.localStorage.getItem(this.resolveTerminalSettingStorageKey(settingKey));
        if (!raw) {
            return this.defaultTerminalSetting();
        }
        try {
            const parsed = JSON.parse(raw) as Partial<TerminalSetting>;
            return {
                themeId: parsed.themeId ?? DEFAULT_TERMINAL_THEME_ID,
                fontSize: normalizeTerminalFontSizeRem(Number(parsed.fontSize)),
                fontFamily: parsed.fontFamily ?? DEFAULT_TERMINAL_FONT_ID,
                cursorStyle: normalizeTerminalCursorStyle(parsed.cursorStyle),
            };
        } catch {
            return this.defaultTerminalSetting();
        }
    }

    saveTerminalSetting(settingKey: string, setting: TerminalSetting): void {
        window.localStorage.setItem(this.resolveTerminalSettingStorageKey(settingKey), JSON.stringify(setting));
    }

    selectBackgroundHintDone(): boolean {
        const raw = window.localStorage.getItem(this.BACKGROUND_HINT_STORAGE_KEY);
        if (raw === null) {
            return false;
        }
        if (raw === this.BACKGROUND_HINT_DONE_VALUE) {
            return true;
        }
        throw new Error(`argus:session:backgroundHint:done invalid: ${raw}`);
    }

    saveBackgroundHintDone(): void {
        window.localStorage.setItem(this.BACKGROUND_HINT_STORAGE_KEY, this.BACKGROUND_HINT_DONE_VALUE);
    }

    selectAppThemeId(): string {
        const raw = window.localStorage.getItem(this.APP_THEME_STORAGE_KEY);
        if (raw === null) {
            return APP_THEME.DEFAULT;
        }
        if (APP_THEME_OPTIONS.some(theme => theme.id === raw)) {
            return raw;
        }
        const legacyTheme = TERMINAL_THEME_OPTIONS.find(theme => theme.id === raw);
        if (legacyTheme) {
            const migrated = ['daybreak', 'seasalt', 'kanagawa-lotus', 'cb-light'].includes(raw) ? APP_THEME.LIGHT : APP_THEME.DEFAULT;
            this.saveAppThemeId(migrated);
            return migrated;
        }
        throw new Error(`argus:app:theme invalid: ${raw}`);
    }

    saveAppThemeId(themeId: string): void {
        window.localStorage.setItem(this.APP_THEME_STORAGE_KEY, themeId);
    }

    private defaultTerminalSetting(): TerminalSetting {
        return {
            themeId: DEFAULT_TERMINAL_THEME_ID,
            fontSize: DEFAULT_TERMINAL_FONT_SIZE,
            fontFamily: DEFAULT_TERMINAL_FONT_ID,
            cursorStyle: DEFAULT_TERMINAL_CURSOR_STYLE,
        };
    }

    private resolveTerminalSettingStorageKey(settingKey: string): string {
        return `${this.TERMINAL_SETTING_STORAGE_PREFIX}${settingKey}`;
    }

    async saveSuggest(enabled: boolean, count: number): Promise<void> {
        await this.request<null>(this.buildUrl('/save_suggest'), {
            method: 'POST',
            headers: this.JSON_HEADERS,
            body: JSON.stringify({enabled, count}),
        });
    }

    async saveCodeIndex(ignore: string | null, includeTests: boolean): Promise<void> {
        await this.request<null>(this.buildUrl('/save_code_index'), {
            method: 'POST',
            headers: this.JSON_HEADERS,
            body: JSON.stringify({ignore, includeTests}),
        });
    }

    async saveSystem(systemLanguage: number, localShellId: string | null, shellCpuPercent: number): Promise<void> {
        await this.request<null>(this.buildUrl('/save_system'), {
            method: 'POST',
            headers: this.JSON_HEADERS,
            body: JSON.stringify({systemLanguage, localShellId, shellCpuPercent}),
        });
    }

    async saveJobCollect(execTimeoutSecs: number, workerParallelism: number, indicatorValueCacheSize: number): Promise<void> {
        await this.request<null>(this.buildUrl('/save_job_collect'), {
            method: 'POST',
            headers: this.JSON_HEADERS,
            body: JSON.stringify({execTimeoutSecs, workerParallelism, indicatorValueCacheSize}),
        });
    }

    async saveJobSshPool(maxSize: number, idleSecs: number): Promise<void> {
        await this.request<null>(this.buildUrl('/save_job_ssh_pool'), {
            method: 'POST',
            headers: this.JSON_HEADERS,
            body: JSON.stringify({maxSize, idleSecs}),
        });
    }

    async saveAuditPolicy(auditPolicy: string | null): Promise<void> {
        await this.request<null>(this.buildUrl('/save_audit_policy'), {
            method: 'POST',
            headers: this.JSON_HEADERS,
            body: JSON.stringify({auditPolicy}),
        });
    }

    async saveJobAlert(aiProvideId: string | null): Promise<void> {
        await this.request<null>(this.buildUrl('/save_job_alert'), {
            method: 'POST',
            headers: this.JSON_HEADERS,
            body: JSON.stringify({aiProvideId}),
        });
    }

    async saveJobDiscover(aiProvideId: string | null, parallelism: number): Promise<void> {
        await this.request<null>(this.buildUrl('/save_job_discover'), {
            method: 'POST',
            headers: this.JSON_HEADERS,
            body: JSON.stringify({aiProvideId, parallelism}),
        });
    }

    async saveRoleModel(role: ModelRole, useCloud: boolean, aiProvideId: string | null, dim: number | null): Promise<void> {
        await this.request<null>(this.buildUrl('/save_role_model'), {
            method: 'POST',
            headers: this.JSON_HEADERS,
            body: JSON.stringify({role, useCloud, aiProvideId, dim}),
        });
    }

    async saveNotify(req: SaveNotifyReq): Promise<void> {
        await this.request<null>(this.buildUrl('/save_notify'), {
            method: 'POST',
            headers: this.JSON_HEADERS,
            body: JSON.stringify(req),
        });
    }

    async saveCloseAction(closeAction: number): Promise<void> {
        await this.request<null>(this.buildUrl('/save_close_action'), {
            method: 'POST',
            headers: this.JSON_HEADERS,
            body: JSON.stringify({closeAction}),
        });
    }
}
