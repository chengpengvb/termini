import {injectable} from "inversify";
import {API_BASE_URL} from "@/runtime-config.ts";
import {getCurrentWindow} from "@tauri-apps/api/window";
import type {HapticRequest} from "@/common/model/dto/Haptic.ts";
import {SystemUtilService} from "@/service/SystemUtilService.ts";
import {BaseServiceImpl} from "@/service/impl/BaseServiceImpl.ts";
import {RespPlatformInfo} from "@/common/model/resp/RespPlatformInfo";
import {BaseApiServiceImpl} from "@/service/impl/BaseApiServiceImpl.ts";
import type {RespDiagnosticsExport} from "@/common/model/resp/RespDiagnostics.ts";
import {BizError, ResultModel, SUCCESS_CODE} from "@/common/model/resp/ResultModel.ts";

@injectable()
export class SystemUtilServiceImpl extends BaseServiceImpl implements SystemUtilService {
    private platformInfoPromise: Promise<RespPlatformInfo> | null = null;

    async getPlatformInfo(): Promise<RespPlatformInfo> {
        if (!this.platformInfoPromise) {
            this.platformInfoPromise = this.fetchPlatformInfo().catch(error => {
                this.platformInfoPromise = null;
                throw error;
            });
        }
        return this.platformInfoPromise;
    }

    async readClipboardLocalPaths(): Promise<string[]> {
        const res = await fetch(`${API_BASE_URL}/system/clipboard/local-paths`, {method: 'GET', headers: BaseApiServiceImpl.buildAuthHeaders()});
        if (!res.ok) {
            throw new Error(`HTTP ${res.status}`);
        }
        const body = (await res.json()) as ResultModel<string[]>;
        if (body.code !== SUCCESS_CODE) {
            throw new BizError(body.code);
        }
        if (!body.data) {
            return [];
        }
        return body.data;
    }

    async readClipboardText(): Promise<string> {
        const res = await fetch(`${API_BASE_URL}/system/clipboard/text`, {method: 'GET', headers: BaseApiServiceImpl.buildAuthHeaders()});
        if (!res.ok) {
            throw new Error(`HTTP ${res.status}`);
        }
        const body = (await res.json()) as ResultModel<string>;
        if (body.code !== SUCCESS_CODE) {
            throw new BizError(body.code);
        }
        if (!body.data) {
            return '';
        }
        return body.data;
    }

    async getMasterKeyPhrase(): Promise<string> {
        const res = await fetch(`${API_BASE_URL}/system/master_key/phrase`, {method: 'GET', headers: BaseApiServiceImpl.buildAuthHeaders()});
        if (!res.ok) {
            throw new Error(`HTTP ${res.status}`);
        }
        const body = (await res.json()) as ResultModel<string>;
        if (body.code !== SUCCESS_CODE) {
            throw new BizError(body.code);
        }
        if (!body.data) {
            throw new Error('master key phrase missing');
        }
        return body.data;
    }

    async getAutoStart(): Promise<boolean> {
        const res = await fetch(`${API_BASE_URL}/system/auto_start`, {method: 'GET', headers: BaseApiServiceImpl.buildAuthHeaders()});
        if (!res.ok) {
            throw new Error(`HTTP ${res.status}`);
        }
        const body = (await res.json()) as ResultModel<boolean>;
        if (body.code !== SUCCESS_CODE) {
            throw new BizError(body.code);
        }
        return !!body.data;
    }

    async saveAutoStart(enabled: boolean): Promise<void> {
        const res = await fetch(`${API_BASE_URL}/system/auto_start`, {
            method: 'POST',
            headers: {'Content-Type': 'application/json', ...BaseApiServiceImpl.buildAuthHeaders()},
            body: JSON.stringify({enabled}),
        });
        if (!res.ok) {
            throw new Error(`HTTP ${res.status}`);
        }
        const body = (await res.json()) as ResultModel<null>;
        if (body.code !== SUCCESS_CODE) {
            throw new BizError(body.code);
        }
    }

    async saveOpenHere(enabled: boolean): Promise<void> {
        const res = await fetch(`${API_BASE_URL}/system/open_here`, {
            method: 'POST',
            headers: {'Content-Type': 'application/json', ...BaseApiServiceImpl.buildAuthHeaders()},
            body: JSON.stringify({enabled}),
        });
        if (!res.ok) {
            throw new Error(`HTTP ${res.status}`);
        }
        const body = (await res.json()) as ResultModel<null>;
        if (body.code !== SUCCESS_CODE) {
            throw new BizError(body.code);
        }
    }

    async writeClipboardText(text: string): Promise<void> {
        const res = await fetch(`${API_BASE_URL}/system/clipboard/text`, {
            method: 'POST',
            headers: {'Content-Type': 'application/json', ...BaseApiServiceImpl.buildAuthHeaders()},
            body: JSON.stringify({text}),
        });
        if (!res.ok) {
            throw new Error(`HTTP ${res.status}`);
        }
        const body = (await res.json()) as ResultModel<null>;
        if (body.code !== SUCCESS_CODE) {
            throw new BizError(body.code);
        }
    }

    async openUrl(url: string): Promise<void> {
        const res = await fetch(`${API_BASE_URL}/system/open_url`, {
            method: 'POST',
            headers: {'Content-Type': 'application/json', ...BaseApiServiceImpl.buildAuthHeaders()},
            body: JSON.stringify({url}),
        });
        if (!res.ok) {
            throw new Error(`HTTP ${res.status}`);
        }
        const body = (await res.json()) as ResultModel<null>;
        if (body.code !== SUCCESS_CODE) {
            throw new BizError(body.code);
        }
    }

    async openLocalPath(path: string): Promise<void> {
        await this.postLocalPath('/system/open_path', path);
    }

    async revealLocalPath(path: string): Promise<void> {
        await this.postLocalPath('/system/reveal_path', path);
    }

    async copyLocalInto(destDir: string, sources: string[]): Promise<void> {
        const res = await fetch(`${API_BASE_URL}/system/copy_local_into`, {
            method: 'POST',
            headers: {'Content-Type': 'application/json', ...BaseApiServiceImpl.buildAuthHeaders()},
            body: JSON.stringify({destDir, sources}),
        });
        if (!res.ok) {
            throw new Error(`HTTP ${res.status}`);
        }
        const body = (await res.json()) as ResultModel<null>;
        if (body.code !== SUCCESS_CODE) {
            if (body.message) {
                throw new Error(body.message);
            }
            throw new BizError(body.code);
        }
    }

    private async postLocalPath(endpoint: string, path: string): Promise<void> {
        const res = await fetch(`${API_BASE_URL}${endpoint}`, {
            method: 'POST',
            headers: {'Content-Type': 'application/json', ...BaseApiServiceImpl.buildAuthHeaders()},
            body: JSON.stringify({path}),
        });
        if (!res.ok) {
            throw new Error(`HTTP ${res.status}`);
        }
        const body = (await res.json()) as ResultModel<null>;
        if (body.code !== SUCCESS_CODE) {
            throw new BizError(body.code);
        }
    }

    async haptic(request: HapticRequest): Promise<void> {
        const res = await fetch(`${API_BASE_URL}/system/haptic`, {
            method: 'POST',
            headers: {'Content-Type': 'application/json', ...BaseApiServiceImpl.buildAuthHeaders()},
            body: JSON.stringify(request),
        });
        if (!res.ok) {
            throw new Error(`HTTP ${res.status}`);
        }
        const body = (await res.json()) as ResultModel<null>;
        if (body.code !== SUCCESS_CODE) {
            throw new BizError(body.code);
        }
    }

    async minimizeWindow(): Promise<void> {
        await this.postWindowAction('/system/window/minimize');
    }

    async toggleMaximizeWindow(): Promise<void> {
        await this.postWindowAction('/system/window/toggle_maximize');
    }

    async notify(title: string, body: string): Promise<void> {
        const res = await fetch(`${API_BASE_URL}/system/notify`, {
            method: 'POST',
            headers: {'Content-Type': 'application/json', ...BaseApiServiceImpl.buildAuthHeaders()},
            body: JSON.stringify({title, body}),
        });
        if (!res.ok) {
            throw new Error(`HTTP ${res.status}`);
        }
        const result = (await res.json()) as ResultModel<null>;
        if (result.code !== SUCCESS_CODE) {
            throw new BizError(result.code);
        }
    }

    async exportDiagnostics(path: string): Promise<RespDiagnosticsExport> {
        const res = await fetch(`${API_BASE_URL}/system/diagnostics/export`, {
            method: 'POST',
            headers: {'Content-Type': 'application/json', ...BaseApiServiceImpl.buildAuthHeaders()},
            body: JSON.stringify({path}),
        });
        if (!res.ok) {
            throw new Error(`HTTP ${res.status}`);
        }
        const result = (await res.json()) as ResultModel<RespDiagnosticsExport>;
        if (result.code !== SUCCESS_CODE || result.data == null) {
            throw new BizError(result.code);
        }
        return result.data;
    }

    async requestUserAttention(active: boolean): Promise<void> {
        const res = await fetch(`${API_BASE_URL}/system/window/request_attention`, {
            method: 'POST',
            headers: {'Content-Type': 'application/json', ...BaseApiServiceImpl.buildAuthHeaders()},
            body: JSON.stringify({active}),
        });
        if (!res.ok) {
            throw new Error(`HTTP ${res.status}`);
        }
        const body = (await res.json()) as ResultModel<null>;
        if (body.code !== SUCCESS_CODE) {
            throw new BizError(body.code);
        }
    }

    async closeWindow(action?: number): Promise<void> {
        const res = await fetch(`${API_BASE_URL}/system/window/close`, {
            method: 'POST',
            headers: {'Content-Type': 'application/json', ...BaseApiServiceImpl.buildAuthHeaders()},
            body: JSON.stringify({action: action ?? null}),
        });
        if (!res.ok) {
            throw new Error(`HTTP ${res.status}`);
        }
        const body = (await res.json()) as ResultModel<null>;
        if (body.code !== SUCCESS_CODE) {
            throw new BizError(body.code);
        }
    }

    async startWindowDragging(): Promise<void> {
        await getCurrentWindow().startDragging();
    }

    async getWindowMaximized(): Promise<boolean> {
        const res = await fetch(`${API_BASE_URL}/system/window/maximized`, {method: 'GET', headers: BaseApiServiceImpl.buildAuthHeaders()});
        if (!res.ok) {
            throw new Error(`HTTP ${res.status}`);
        }
        const body = (await res.json()) as ResultModel<boolean>;
        if (body.code !== SUCCESS_CODE) {
            throw new BizError(body.code);
        }
        return !!body.data;
    }

    async setWindowFullscreen(fullscreen: boolean): Promise<void> {
        const res = await fetch(`${API_BASE_URL}/system/window/fullscreen`, {
            method: 'POST',
            headers: {'Content-Type': 'application/json', ...BaseApiServiceImpl.buildAuthHeaders()},
            body: JSON.stringify({fullscreen}),
        });
        if (!res.ok) {
            throw new Error(`HTTP ${res.status}`);
        }
        const body = (await res.json()) as ResultModel<null>;
        if (body.code !== SUCCESS_CODE) {
            throw new BizError(body.code);
        }
    }

    async setScreenOrientation(lock: string): Promise<void> {
        const native = window.TerminiNative;
        if (!native) {
            throw new Error('TerminiNative.setScreenOrientation unavailable');
        }
        native.setScreenOrientation(lock);
    }

    async getWindowFullscreen(): Promise<boolean> {
        const res = await fetch(`${API_BASE_URL}/system/window/fullscreen`, {method: 'GET', headers: BaseApiServiceImpl.buildAuthHeaders()});
        if (!res.ok) {
            throw new Error(`HTTP ${res.status}`);
        }
        const body = (await res.json()) as ResultModel<boolean>;
        if (body.code !== SUCCESS_CODE) {
            throw new BizError(body.code);
        }
        return !!body.data;
    }

    private async postWindowAction(path: string): Promise<void> {
        const res = await fetch(`${API_BASE_URL}${path}`, {method: 'POST', headers: BaseApiServiceImpl.buildAuthHeaders()});
        if (!res.ok) {
            throw new Error(`HTTP ${res.status}`);
        }
        const body = (await res.json()) as ResultModel<null>;
        if (body.code !== SUCCESS_CODE) {
            throw new BizError(body.code);
        }
    }

    private async fetchPlatformInfo(): Promise<RespPlatformInfo> {
        const controller = new AbortController();
        const timer = window.setTimeout(() => controller.abort(), 1500);
        try {
            const res = await fetch(`${API_BASE_URL}/system/platform/info`, {method: 'GET', signal: controller.signal, headers: BaseApiServiceImpl.buildAuthHeaders()});
            if (!res.ok) {
                throw new Error(`HTTP ${res.status}`);
            }
            const body = (await res.json()) as ResultModel<RespPlatformInfo>;
            if (body.code !== SUCCESS_CODE) {
                throw new BizError(body.code);
            }
            if (!body.data) {
                throw new Error('platform info missing');
            }
            return body.data;
        } finally {
            window.clearTimeout(timer);
        }
    }

}