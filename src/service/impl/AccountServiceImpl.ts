import {inject, injectable} from 'inversify';
import {API_BASE_URL} from '@/runtime-config.ts';
import type {AccountService} from '../AccountService.ts';
import {BaseListenerService} from './BaseListenerService.ts';
import {ENTITY_TYPE} from '@/common/const/EntityTypeConst.ts';
import {STORE_PRO_PRODUCT_ID} from '@/common/const/StoreConst.ts';
import type {ResultModel} from '@/common/model/resp/ResultModel.ts';
import {BaseApiServiceImpl} from '@/service/impl/BaseApiServiceImpl.ts';
import type {UserAccount} from '@/common/model/resp/RespUserAccount.ts';
import {WebSocketServiceImpl} from '@/service/impl/WebSocketServiceImpl.ts';
import type {RespSocketContext} from '@/common/model/resp/RespWsContext.ts';
import type {AboutInfo, AboutSiteKind} from '@/common/model/resp/RespAppVersion.ts';
import {isAccountEvent, RESP_EVENT_ACCOUNT} from '@/common/model/resp/event/RespEventAccount.ts';
import {CLOUD_SYNC_PASSWORD_MAX_LEN, CLOUD_SYNC_PASSWORD_MIN_LEN} from '@/common/model/dto/CloudSync.ts';
import type {CloudSyncChoice, CloudSyncConflict, CloudSyncStatus} from '@/common/model/dto/CloudSync.ts';
import {BizError, RESULT_CODE, SUCCESS_CODE, resolveLoginErrorPayload} from '@/common/model/resp/ResultModel.ts';

@injectable()
export class AccountServiceImpl extends BaseListenerService implements AccountService {
    private revalidating = false;
    private lastFocusCheckAt = 0;
    private cloudSyncing: number | null = null;
    private static readonly SYNC_KEY = 'cloud_sync';
    private static readonly LISTENER_KEY = 'account';
    private static readonly ONBOARD_DONE_VALUE = '1';
    private static readonly LOGIN_ERROR_KEY = 'login_error';
    private account: UserAccount | null | undefined = undefined;
    private static readonly FOCUS_CHECK_MIN_INTERVAL_MS = 30000;
    private static readonly FEEDBACK_COOLDOWN_MS = 30 * 60 * 1000;
    private static readonly ONBOARD_DONE_KEY = 'argus:onboard:done';
    private static readonly DOWNLOADED_KEY = 'cloud_sync_downloaded';
    private static readonly FEEDBACK_LAST_KEY = 'termini:feedback:last';
    private static readonly PROFILE_CACHE_KEY = 'argus:account:profile';
    private static readonly DECK_INPUT_DONE_KEY = 'argus:onboard:deck-input';
    private static readonly SESSION_VALIDATED_KEY = 'termini:account:session_validated';

    constructor(
        @inject(WebSocketServiceImpl) private readonly webSocketService: WebSocketServiceImpl,
    ) {
        super();
        this.webSocketService.addWsListener((context: RespSocketContext) => {
            this.handleRespContext(context);
        });
        BaseApiServiceImpl.setUnauthorizedHandler(() => {
            this.handleUnauthorized();
        });
    }

    protected getNotifyEntityType(): number {
        return ENTITY_TYPE.NONE;
    }

    private handleUnauthorized(): void {
        if (this.account == null || this.revalidating) {
            return;
        }
        this.revalidating = true;
        void this.fetchProfile()
            .then((account) => {
                this.applyAccount(account);
            })
            .catch((error: unknown) => {
                if (!AccountServiceImpl.isAuthError(error)) {
                    console.error('[account] local session rebuild failed, keeping current login state', error);
                    return;
                }
                AccountServiceImpl.clearProfileCache();
                this.applyAccount(null);
            })
            .finally(() => {
                this.revalidating = false;
            });
    }

    getPhase(): number {
        return 300;
    }

    override async start(): Promise<void> {
        window.addEventListener('focus', () => {
            this.onWindowFocus();
        });
        void this.pullAccount();
    }

    private async pullAccount(): Promise<void> {
        if (BaseApiServiceImpl.buildAuthHeaders().Authorization === undefined) {
            this.applyAccount(null);
            return;
        }
        let account: UserAccount | null;
        try {
            account = await this.fetchProfile();
        } catch (error) {
            console.error('[account] fetch account failed on start', error);
            account = null;
        }
        this.applyAccount(account);
    }

    private applyAccount(account: UserAccount | null): void {
        this.account = account;
        this.notifyListeners<UserAccount | null>(AccountServiceImpl.LISTENER_KEY, account);
    }

    private static isAuthError(error: unknown): boolean {
        if (!(error instanceof BizError)) {
            return false;
        }
        return error.code === RESULT_CODE.NOT_LOGGED_IN || error.code === RESULT_CODE.UNAUTHORIZED;
    }

    private onWindowFocus(): void {
        if (this.account == null) {
            return;
        }
        const now = Date.now();
        if (now - this.lastFocusCheckAt < AccountServiceImpl.FOCUS_CHECK_MIN_INTERVAL_MS) {
            return;
        }
        this.lastFocusCheckAt = now;
        void this.refreshAccount().catch((error: unknown) => {
            if (AccountServiceImpl.isAuthError(error)) {
                this.handleUnauthorized();
                return;
            }
            console.error('[account] account verification on window focus failed', error);
        });
    }

    private static clearProfileCache(): void {
        window.localStorage.removeItem(AccountServiceImpl.PROFILE_CACHE_KEY);
        window.sessionStorage.removeItem(AccountServiceImpl.SESSION_VALIDATED_KEY);
    }

    async login(): Promise<void> {
        const res = await fetch(`${API_BASE_URL}/account/login_url`);
        const body = (await res.json()) as ResultModel<{ url: string }>;
        if (body.code !== SUCCESS_CODE || !body.data?.url) {
            throw new BizError(body.code);
        }
    }

    async loginWithPassword(email: string, password: string): Promise<void> {
        await this.postAccount('/account/client_login', {email, password});
    }

    async sendRegisterCode(email: string): Promise<void> {
        await this.postAccount('/account/register_code', {email});
    }

    async register(email: string, password: string, code: string): Promise<void> {
        await this.postAccount('/account/register', {email, password, code});
    }

    async sendResetCode(email: string): Promise<void> {
        await this.postAccount('/account/reset_code', {email});
    }

    async resetPassword(email: string, password: string, code: string): Promise<void> {
        await this.postAccount('/account/reset_password', {email, password, code});
    }

    async sendDeleteCode(): Promise<void> {
        await this.postAuthed('/account/delete_code', {});
    }

    async deleteAccount(code: string): Promise<void> {
        await this.postAuthed('/account/delete_account', {code});
        AccountServiceImpl.clearProfileCache();
        this.applyAccount(null);
    }

    async openPortal(): Promise<void> {
        const res = await fetch(`${API_BASE_URL}/account/portal_url`);
        const body = (await res.json()) as ResultModel<{ url: string }>;
        if (body.code !== SUCCESS_CODE || !body.data?.url) {
            throw new BizError(body.code);
        }
    }

    async about(): Promise<AboutInfo> {
        const res = await fetch(`${API_BASE_URL}/account/about`);
        const body = (await res.json()) as ResultModel<AboutInfo>;
        if (body.code !== SUCCESS_CODE || !body.data) {
            throw new BizError(body.code, body.message);
        }
        return body.data;
    }

    async openSite(kind: AboutSiteKind): Promise<void> {
        const res = await fetch(`${API_BASE_URL}/account/open_site?kind=${kind}`);
        const body = (await res.json()) as ResultModel<void>;
        if (body.code !== SUCCESS_CODE) {
            throw new BizError(body.code);
        }
    }

    feedbackCooldownLeftMs(): number {
        const raw = window.localStorage.getItem(AccountServiceImpl.FEEDBACK_LAST_KEY);
        if (raw === null) {
            return 0;
        }
        const last = Number(raw);
        if (!Number.isFinite(last)) {
            window.localStorage.removeItem(AccountServiceImpl.FEEDBACK_LAST_KEY);
            return 0;
        }
        const left = last + AccountServiceImpl.FEEDBACK_COOLDOWN_MS - Date.now();
        return left > 0 ? left : 0;
    }

    async submitFeedback(content: string): Promise<void> {
        const res = await fetch(`${API_BASE_URL}/account/feedback`, {
            method: 'POST',
            headers: {'Content-Type': 'application/json'},
            body: JSON.stringify({content}),
        });
        const body = (await res.json()) as ResultModel<void>;
        if (body.code !== SUCCESS_CODE) {
            throw new BizError(body.code);
        }
        window.localStorage.setItem(AccountServiceImpl.FEEDBACK_LAST_KEY, String(Date.now()));
    }

    async logout(): Promise<void> {
        const res = await fetch(`${API_BASE_URL}/account/logout`, {
            method: 'DELETE',
            headers: BaseApiServiceImpl.buildAuthHeaders(),
        });
        const body = (await res.json()) as ResultModel<null>;
        if (body.code !== SUCCESS_CODE) {
            throw new BizError(body.code);
        }
        AccountServiceImpl.clearProfileCache();
        this.applyAccount(null);
    }

    async refreshAccount(): Promise<void> {
        this.applyAccount(await this.fetchProfile());
    }

    async subscribePro(): Promise<void> {
        const res = await fetch(`${API_BASE_URL}/store/subscribe`, {
            method: 'POST',
            headers: {...BaseApiServiceImpl.buildAuthHeaders(), 'Content-Type': 'application/json'},
            body: JSON.stringify({productId: STORE_PRO_PRODUCT_ID}),
        });
        const body = (await res.json()) as ResultModel<null>;
        if (body.code !== SUCCESS_CODE) {
            throw new BizError(body.code, body.message);
        }
        await this.refreshAccount();
    }

    async restorePro(): Promise<void> {
        const res = await fetch(`${API_BASE_URL}/store/restore`, {
            method: 'POST',
            headers: BaseApiServiceImpl.buildAuthHeaders(),
        });
        const body = (await res.json()) as ResultModel<null>;
        if (body.code !== SUCCESS_CODE) {
            throw new BizError(body.code, body.message);
        }
        await this.refreshAccount();
    }

    selectOnboardDone(): boolean {
        const raw = window.localStorage.getItem(AccountServiceImpl.ONBOARD_DONE_KEY);
        if (raw === null) {
            return false;
        }
        if (raw === AccountServiceImpl.ONBOARD_DONE_VALUE) {
            return true;
        }
        throw new Error(`argus:onboard:done invalid: ${raw}`);
    }

    saveOnboardDone(): void {
        window.localStorage.setItem(AccountServiceImpl.ONBOARD_DONE_KEY, AccountServiceImpl.ONBOARD_DONE_VALUE);
    }

    selectDeckInputOnboardDone(): boolean {
        const raw = window.localStorage.getItem(AccountServiceImpl.DECK_INPUT_DONE_KEY);
        if (raw === null) {
            return false;
        }
        if (raw === AccountServiceImpl.ONBOARD_DONE_VALUE) {
            return true;
        }
        throw new Error(`argus:onboard:deck-input invalid: ${raw}`);
    }

    saveDeckInputOnboardDone(): void {
        window.localStorage.setItem(AccountServiceImpl.DECK_INPUT_DONE_KEY, AccountServiceImpl.ONBOARD_DONE_VALUE);
    }

    getAccount(): UserAccount | null | undefined {
        return this.account;
    }

    addAccountChangeListener(cb: (account: UserAccount | null) => void): () => void {
        return this.addListener<UserAccount | null>(AccountServiceImpl.LISTENER_KEY, cb);
    }

    async cloudSyncStatus(): Promise<CloudSyncStatus> {
        const res = await fetch(`${API_BASE_URL}/cloud_sync/status`, {
            headers: BaseApiServiceImpl.buildAuthHeaders(),
        });
        const body = (await res.json()) as ResultModel<CloudSyncStatus>;
        if (body.code !== SUCCESS_CODE || body.data == null) {
            throw new BizError(body.code);
        }
        return body.data;
    }

    async prepareCloudSync(op: number, password: string): Promise<CloudSyncConflict[]> {
        this.ensurePassword(password);
        if (this.cloudSyncing != null) {
            throw new BizError(RESULT_CODE.CLOUD_SYNC_BUSY);
        }
        this.setSyncing(op);
        try {
            const data = await this.postCloudSync<{ conflicts: CloudSyncConflict[] }>('/cloud_sync/prepare', {op, password});
            if (data === null) {
                throw new BizError(RESULT_CODE.CLOUD_SYNC_NO_DATA);
            }
            return data.conflicts;
        } catch (e) {
            this.setSyncing(null);
            throw e;
        }
    }

    async applyCloudSync(op: number, password: string, choices: CloudSyncChoice[]): Promise<void> {
        try {
            await this.postCloudSync<null>('/cloud_sync/apply', {op, password, choices});
            this.notifyListeners<void>(AccountServiceImpl.DOWNLOADED_KEY, undefined);
        } finally {
            this.setSyncing(null);
        }
    }

    cancelCloudSync(): void {
        this.setSyncing(null);
    }

    getCloudSyncing(): number | null {
        return this.cloudSyncing;
    }

    addCloudSyncingListener(cb: (op: number | null) => void): () => void {
        return this.addListener<number | null>(AccountServiceImpl.SYNC_KEY, cb);
    }

    addCloudSyncDownloadedListener(cb: () => void): () => void {
        return this.addListener<void>(AccountServiceImpl.DOWNLOADED_KEY, cb);
    }

    addLoginErrorListener(cb: (message: string) => void): () => void {
        return this.addListener<string>(AccountServiceImpl.LOGIN_ERROR_KEY, cb);
    }

    private setSyncing(op: number | null): void {
        this.cloudSyncing = op;
        this.notifyListeners<number | null>(AccountServiceImpl.SYNC_KEY, op);
    }

    private ensurePassword(password: string): void {
        if (password.length < CLOUD_SYNC_PASSWORD_MIN_LEN) {
            throw new BizError(RESULT_CODE.PASSWORD_TOO_SHORT);
        }
        if (password.length > CLOUD_SYNC_PASSWORD_MAX_LEN) {
            throw new BizError(RESULT_CODE.PASSWORD_TOO_LONG);
        }
    }

    private async postAccount(path: string, body: unknown): Promise<void> {
        const res = await fetch(`${API_BASE_URL}${path}`, {
            method: 'POST',
            headers: {'Content-Type': 'application/json'},
            body: JSON.stringify(body),
        });
        const result = (await res.json()) as ResultModel<void>;
        if (result.code !== SUCCESS_CODE) {
            throw new BizError(result.code, result.message);
        }
    }

    private async postAuthed(path: string, body: unknown): Promise<void> {
        const res = await fetch(`${API_BASE_URL}${path}`, {
            method: 'POST',
            headers: {...BaseApiServiceImpl.buildAuthHeaders(), 'Content-Type': 'application/json'},
            body: JSON.stringify(body),
        });
        const result = (await res.json()) as ResultModel<void>;
        if (result.code !== SUCCESS_CODE) {
            throw new BizError(result.code, result.message);
        }
    }

    private async postCloudSync<R>(path: string, body: unknown): Promise<R | null> {
        const res = await fetch(`${API_BASE_URL}${path}`, {
            method: 'POST',
            headers: {...BaseApiServiceImpl.buildAuthHeaders(), 'Content-Type': 'application/json'},
            body: JSON.stringify(body),
        });
        const result = (await res.json()) as ResultModel<R>;
        if (result.code !== SUCCESS_CODE) {
            throw new BizError(result.code);
        }
        return result.data ?? null;
    }

    private async requestProfile(): Promise<ResultModel<UserAccount>> {
        const res = await fetch(`${API_BASE_URL}/account/profile`, {
            headers: BaseApiServiceImpl.buildAuthHeaders(),
        });
        return (await res.json()) as ResultModel<UserAccount>;
    }

    private acceptProfile(body: ResultModel<UserAccount>): UserAccount {
        if (body.code !== SUCCESS_CODE || !body.data) {
            throw new BizError(body.code);
        }
        window.localStorage.setItem(AccountServiceImpl.PROFILE_CACHE_KEY, JSON.stringify(body.data));
        window.sessionStorage.setItem(AccountServiceImpl.SESSION_VALIDATED_KEY, '1');
        return body.data;
    }

    private async fetchProfile(): Promise<UserAccount> {
        if (BaseApiServiceImpl.buildAuthHeaders().Authorization === undefined) {
            throw new BizError(RESULT_CODE.UNAUTHORIZED);
        }
        return this.acceptProfile(await this.requestProfile());
    }

    private handleRespContext(context: RespSocketContext): void {
        if (!isAccountEvent(context.type)) {
            return;
        }
        if (context.type === RESP_EVENT_ACCOUNT.ON_LOGIN) {
            BaseApiServiceImpl.setApiToken(context.payload as string);
            void this.fetchProfile().then((account) => {
                this.applyAccount(account);
            });
            return;
        }
        if (context.type === RESP_EVENT_ACCOUNT.ON_LOGOUT) {
            BaseApiServiceImpl.setApiToken(null);
            AccountServiceImpl.clearProfileCache();
            this.applyAccount(null);
            return;
        }
        if (context.type === RESP_EVENT_ACCOUNT.ON_LOGIN_ERROR) {
            this.notifyListeners<string>(AccountServiceImpl.LOGIN_ERROR_KEY, resolveLoginErrorPayload(context.payload as string));
        }
    }
}
