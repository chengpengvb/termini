import type {BaseService} from './BaseService.ts';
import type {UserAccount} from '@/common/model/resp/RespUserAccount.ts';
import type {AboutInfo, AboutSiteKind} from '@/common/model/resp/RespAppVersion.ts';
import type {CloudSyncChoice, CloudSyncConflict, CloudSyncStatus} from '@/common/model/dto/CloudSync.ts';

export interface AccountService extends BaseService {
    login(): Promise<void>;
    logout(): Promise<void>;
    cancelCloudSync(): void;
    saveOnboardDone(): void;
    restorePro(): Promise<void>;
    openPortal(): Promise<void>;
    about(): Promise<AboutInfo>;
    selectOnboardDone(): boolean;
    subscribePro(): Promise<void>;
    sendDeleteCode(): Promise<void>;
    refreshAccount(): Promise<void>;
    saveDeckInputOnboardDone(): void;
    feedbackCooldownLeftMs(): number;
    getCloudSyncing(): number | null;
    selectDeckInputOnboardDone(): boolean;
    cloudSyncStatus(): Promise<CloudSyncStatus>;
    sendResetCode(email: string): Promise<void>;
    deleteAccount(code: string): Promise<void>;
    getAccount(): UserAccount | null | undefined;
    openSite(kind: AboutSiteKind): Promise<void>;
    sendRegisterCode(email: string): Promise<void>;
    submitFeedback(content: string): Promise<void>;
    addCloudSyncDownloadedListener(cb: () => void): () => void;
    addLoginErrorListener(cb: (message: string) => void): () => void;
    loginWithPassword(email: string, password: string): Promise<void>;
    addCloudSyncingListener(cb: (op: number | null) => void): () => void;
    register(email: string, password: string, code: string): Promise<void>;
    resetPassword(email: string, password: string, code: string): Promise<void>;
    prepareCloudSync(op: number, password: string): Promise<CloudSyncConflict[]>;
    addAccountChangeListener(cb: (account: UserAccount | null) => void): () => void;
    applyCloudSync(op: number, password: string, choices: CloudSyncChoice[]): Promise<void>;
}
