import type {ApiService} from './ApiService';
import type {Config, ModelRole} from '@/common/model/Config';
import type {SaveNotifyReq} from '@/common/model/req/ReqConfig.ts';
import type {TerminalSetting} from '@/page/common/util/terminal.ts';

export interface ConfigService extends ApiService<Config> {

    selectAppThemeId(): string;

    saveBackgroundHintDone(): void;
    
    selectBackgroundHintDone(): boolean;

    saveAppThemeId(themeId: string): void;

    selectConfig(): Promise<Config | null>;

    saveNotify(req: SaveNotifyReq): Promise<void>;

    saveCloseAction(closeAction: number): Promise<void>;

    saveJobAlert(aiProvideId: string | null): Promise<void>;

    saveAuditPolicy(auditPolicy: string | null): Promise<void>;

    selectTerminalSetting(settingKey: string): TerminalSetting;

    saveSuggest(enabled: boolean, count: number): Promise<void>;

    saveJobSshPool(maxSize: number, idleSecs: number): Promise<void>;

    saveTerminalSetting(settingKey: string, setting: TerminalSetting): void;

    saveCodeIndex(ignore: string | null, includeTests: boolean): Promise<void>;

    saveJobDiscover(aiProvideId: string | null, parallelism: number): Promise<void>;

    saveSystem(systemLanguage: number, localShellId: string | null, shellCpuPercent: number): Promise<void>;

    saveRoleModel(role: ModelRole, useCloud: boolean, aiProvideId: string | null, dim: number | null): Promise<void>;

    saveJobCollect(execTimeoutSecs: number, workerParallelism: number, indicatorValueCacheSize: number): Promise<void>;

}