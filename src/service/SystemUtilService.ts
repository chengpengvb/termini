import {BaseService} from "@/service/BaseService.ts";
import type {HapticRequest} from "@/common/model/dto/Haptic.ts";
import {RespPlatformInfo} from "@/common/model/resp/RespPlatformInfo.ts";
import type {RespDiagnosticsExport} from "@/common/model/resp/RespDiagnostics.ts";

export interface SystemUtilService extends BaseService {
    minimizeWindow(): Promise<void>;
    getAutoStart(): Promise<boolean>;
    openUrl(url: string): Promise<void>;
    readClipboardText(): Promise<string>;
    startWindowDragging(): Promise<void>;
    toggleMaximizeWindow(): Promise<void>;
    getMasterKeyPhrase(): Promise<string>;
    getWindowMaximized(): Promise<boolean>;
    getWindowFullscreen(): Promise<boolean>;
    openLocalPath(path: string): Promise<void>;
    closeWindow(action?: number): Promise<void>;
    revealLocalPath(path: string): Promise<void>;
    getPlatformInfo(): Promise<RespPlatformInfo>;
    readClipboardLocalPaths(): Promise<string[]>;
    haptic(request: HapticRequest): Promise<void>;
    saveAutoStart(enabled: boolean): Promise<void>;
    saveOpenHere(enabled: boolean): Promise<void>;
    writeClipboardText(text: string): Promise<void>;
    setScreenOrientation(lock: string): Promise<void>;
    notify(title: string, body: string): Promise<void>;
    requestUserAttention(active: boolean): Promise<void>;
    setWindowFullscreen(fullscreen: boolean): Promise<void>;
    exportDiagnostics(path: string): Promise<RespDiagnosticsExport>;
    copyLocalInto(destDir: string, sources: string[]): Promise<void>;
}
