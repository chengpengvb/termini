import {translate} from '@/common/i18n/i18n.ts';

export const SUCCESS_CODE = 200;
export const ERROR_CODE = 500;

export const RESULT_CODE = {
    ERROR: 500,
    SUCCESS: 200,
    UNAUTHORIZED: 401,
    AI_MODEL_UNAVAILABLE: 40001,
    CANCELLED: 40002,
    MAX_TURNS_REACHED: 40003,
    IMAGES_UNSUPPORTED: 40004,
    AI_MODEL_NOT_FOUND: 40005,
    NO_ACCOUNT: 40006,
    NO_BASE_URL: 40007,
    BILLING_RATE: 40008,
    QUOTA: 41009,
    SUBSCRIPTION: 41010,
    USER_DISABLED: 41011,
    DEVICE_LIMIT_EXCEEDED: 41012,
    PRO_REQUIRED: 41013,
    SUGGEST_LIMIT: 41014,
    CHAT_NOT_FOUND: 40011,
    CHAT_TITLE_EMPTY: 40012,
    CHAT_GENERATING: 40013,
    CHAT_ITEM_NOT_FOUND: 40014,
    CHAT_TYPE_UNKNOWN: 40015,
    PROJECT_NOT_FOUND: 40016,
    PROJECT_NAME_EMPTY: 40017,
    JOB_DEVICE_NOT_FOUND: 40018,
    TOKEN_EMPTY: 40019,
    NOT_LOGGED_IN: 40020,
    URL_EMPTY: 40021,
    INVALID_URL: 40022,
    UNSUPPORTED_URL_SCHEME: 40023,
    DEFAULT_CONFIG_NOT_FOUND: 40024,
    LOGIN_URL_EMPTY: 40025,
    SUGGEST_PROVIDE_EMPTY: 40026,
    SUGGEST_COUNT_RANGE: 40027,
    GROUP_NOT_FOUND: 40028,
    GROUP_NAME_EMPTY: 40029,
    GROUP_NAME_EXISTS: 40030,
    CLOUD_SYNC_BUSY: 40031,
    CLOUD_SYNC_OP_INVALID: 40032,
    CLOUD_SYNC_CHOICE_INVALID: 40033,
    PASSWORD_TOO_SHORT: 40034,
    PASSWORD_TOO_LONG: 40035,
    PASSWORD_MISMATCH: 40036,
    CLOUD_SYNC_NOT_FOUND: 40037,
    MCP_SERVER_NOT_FOUND: 40038,
    NAME_EMPTY: 40039,
    MCP_STDIO_COMMAND_REQUIRED: 40040,
    MCP_HTTP_URL_REQUIRED: 40041,
    MCP_TRANSPORT_UNSUPPORTED: 40042,
    TIMEOUT_SECS_INVALID: 40043,
    SKILL_NAME_EMPTY: 40044,
    SKILL_CONTENT_EMPTY: 40045,
    SKILL_DESCRIPTION_TOO_LONG: 40046,
    INDICATOR_DEVICE_REQUIRED: 40047,
    INDICATOR_NAME_EMPTY: 40048,
    INDICATOR_TRANSFORM_INVALID: 40049,
    INDICATOR_DELETED: 40050,
    JOB_DEVICE_ID_REQUIRED: 40051,
    JOB_INDICATOR_ID_REQUIRED: 40052,
    DISCOVER_IN_PROGRESS: 40053,
    DISCOVER_MODEL_NOT_SET: 40054,
    DISCOVER_ENDED: 40055,
    DISCOVER_NOT_READY: 40056,
    DISCOVER_QUEUED_CANCELLED: 40057,
    DISCOVER_COLLECT_CONFIG_MISSING: 40058,
    DEVICE_DELETED: 40059,
    SKILL_TASK_ID_EMPTY: 40060,
    SKILL_TASK_RUNNING: 40061,
    SKILL_TASK_NOT_FOUND: 40062,
    SKILL_EXTRACT_INPUT_REQUIRED: 40063,
    PROVIDER_NOT_FOUND: 40064,
    EXEC_TIMEOUT_INVALID: 40065,
    SKILL_EXTRACT_EMPTY: 40066,
    CLOUD_SYNC_NO_DATA: 40067,
    DEVICE_NOT_FOUND: 40068,
    JSON_ARRAY_REQUIRED: 40069,
    JSON_OBJECT_REQUIRED: 40070,
    CLIPBOARD_WRITE_FAILED: 40071,
    HAPTIC_UNSUPPORTED: 40072,
    WINDOW_OP_UNSUPPORTED: 40073,
    WINDOW_OP_FAILED: 40074,
    MAIN_WINDOW_NOT_FOUND: 40075,
    DRAG_UNSUPPORTED: 40076,
    SESSION_ID_EMPTY: 40077,
    DEVICE_ID_EMPTY: 40078,
    DEVICE_ID_LOCAL: 40079,
    SSH_SHELL_NOT_OPENED: 40080,
    SSH_CONFIG_NOT_FOUND: 40081,
    CONFIG_ID_MISMATCH: 40082,
    SKILL_EXTRACT_PARSE_FAILED: 40083,
    OPEN_URL_FAILED: 40084,
    MODEL_MAX_TOKEN_MISSING: 40085,
    CLIPBOARD_READ_FAILED: 40086,
    CHAT_IMAGE_NOT_FOUND: 40087,
    CHAT_IMAGE_SAVE_FAILED: 40088,
    STREAM_IDLE_TIMEOUT: 40089,
    LOCAL_PATH_NOT_FOUND: 40090,
    OPEN_LOCAL_PATH_FAILED: 40091,
    GROUP_DEFAULT_PROTECTED: 40092,
} as const;

const RESULT_I18N_KEY: Record<number, string> = {
    [RESULT_CODE.ERROR]: 'result.error',
    [RESULT_CODE.CANCELLED]: 'result.cancelled',
    [RESULT_CODE.MAX_TURNS_REACHED]: 'result.maxTurnsReached',
    [RESULT_CODE.IMAGES_UNSUPPORTED]: 'result.imagesUnsupported',
    [RESULT_CODE.AI_MODEL_UNAVAILABLE]: 'result.aiModelUnavailable',
    [RESULT_CODE.AI_MODEL_NOT_FOUND]: 'result.aiModelNotFound',
    [RESULT_CODE.NO_ACCOUNT]: 'result.noAccount',
    [RESULT_CODE.NO_BASE_URL]: 'result.noBaseUrl',
    [RESULT_CODE.BILLING_RATE]: 'result.billingRate',
    [RESULT_CODE.QUOTA]: 'result.quota',
    [RESULT_CODE.SUBSCRIPTION]: 'result.subscription',
    [RESULT_CODE.USER_DISABLED]: 'result.userDisabled',
    [RESULT_CODE.DEVICE_LIMIT_EXCEEDED]: 'result.deviceLimitExceeded',
    [RESULT_CODE.PRO_REQUIRED]: 'result.proRequired',
    [RESULT_CODE.SUGGEST_LIMIT]: 'result.suggestLimit',
    [RESULT_CODE.CHAT_NOT_FOUND]: 'result.chatNotFound',
    [RESULT_CODE.CHAT_TITLE_EMPTY]: 'result.chatTitleEmpty',
    [RESULT_CODE.CHAT_GENERATING]: 'result.chatGenerating',
    [RESULT_CODE.CHAT_ITEM_NOT_FOUND]: 'result.chatItemNotFound',
    [RESULT_CODE.CHAT_TYPE_UNKNOWN]: 'result.chatTypeUnknown',
    [RESULT_CODE.PROJECT_NOT_FOUND]: 'result.projectNotFound',
    [RESULT_CODE.PROJECT_NAME_EMPTY]: 'result.projectNameEmpty',
    [RESULT_CODE.JOB_DEVICE_NOT_FOUND]: 'result.jobDeviceNotFound',
    [RESULT_CODE.TOKEN_EMPTY]: 'result.tokenEmpty',
    [RESULT_CODE.NOT_LOGGED_IN]: 'result.notLoggedIn',
    [RESULT_CODE.UNAUTHORIZED]: 'result.notLoggedIn',
    [RESULT_CODE.URL_EMPTY]: 'result.urlEmpty',
    [RESULT_CODE.INVALID_URL]: 'result.invalidUrl',
    [RESULT_CODE.UNSUPPORTED_URL_SCHEME]: 'result.unsupportedUrlScheme',
    [RESULT_CODE.DEFAULT_CONFIG_NOT_FOUND]: 'result.defaultConfigNotFound',
    [RESULT_CODE.LOGIN_URL_EMPTY]: 'result.loginUrlEmpty',
    [RESULT_CODE.SUGGEST_PROVIDE_EMPTY]: 'result.suggestProvideEmpty',
    [RESULT_CODE.SUGGEST_COUNT_RANGE]: 'result.suggestCountRange',
    [RESULT_CODE.GROUP_NOT_FOUND]: 'result.groupNotFound',
    [RESULT_CODE.GROUP_NAME_EMPTY]: 'result.groupNameEmpty',
    [RESULT_CODE.GROUP_NAME_EXISTS]: 'result.groupNameExists',
    [RESULT_CODE.CLOUD_SYNC_BUSY]: 'result.cloudSyncBusy',
    [RESULT_CODE.CLOUD_SYNC_OP_INVALID]: 'result.cloudSyncOpInvalid',
    [RESULT_CODE.CLOUD_SYNC_CHOICE_INVALID]: 'result.cloudSyncChoiceInvalid',
    [RESULT_CODE.PASSWORD_TOO_SHORT]: 'result.passwordTooShort',
    [RESULT_CODE.PASSWORD_TOO_LONG]: 'result.passwordTooLong',
    [RESULT_CODE.PASSWORD_MISMATCH]: 'result.passwordMismatch',
    [RESULT_CODE.CLOUD_SYNC_NOT_FOUND]: 'result.cloudSyncNotFound',
    [RESULT_CODE.MCP_SERVER_NOT_FOUND]: 'result.mcpServerNotFound',
    [RESULT_CODE.NAME_EMPTY]: 'result.nameEmpty',
    [RESULT_CODE.MCP_STDIO_COMMAND_REQUIRED]: 'result.mcpStdioCommandRequired',
    [RESULT_CODE.MCP_HTTP_URL_REQUIRED]: 'result.mcpHttpUrlRequired',
    [RESULT_CODE.MCP_TRANSPORT_UNSUPPORTED]: 'result.mcpTransportUnsupported',
    [RESULT_CODE.TIMEOUT_SECS_INVALID]: 'result.timeoutSecsInvalid',
    [RESULT_CODE.SKILL_NAME_EMPTY]: 'result.skillNameEmpty',
    [RESULT_CODE.SKILL_CONTENT_EMPTY]: 'result.skillContentEmpty',
    [RESULT_CODE.SKILL_DESCRIPTION_TOO_LONG]: 'result.skillDescriptionTooLong',
    [RESULT_CODE.INDICATOR_DEVICE_REQUIRED]: 'result.indicatorDeviceRequired',
    [RESULT_CODE.INDICATOR_NAME_EMPTY]: 'result.indicatorNameEmpty',
    [RESULT_CODE.INDICATOR_TRANSFORM_INVALID]: 'result.indicatorTransformInvalid',
    [RESULT_CODE.INDICATOR_DELETED]: 'result.indicatorDeleted',
    [RESULT_CODE.JOB_DEVICE_ID_REQUIRED]: 'result.jobDeviceIdRequired',
    [RESULT_CODE.JOB_INDICATOR_ID_REQUIRED]: 'result.jobIndicatorIdRequired',
    [RESULT_CODE.DISCOVER_IN_PROGRESS]: 'result.discoverInProgress',
    [RESULT_CODE.DISCOVER_MODEL_NOT_SET]: 'result.discoverModelNotSet',
    [RESULT_CODE.DISCOVER_ENDED]: 'result.discoverEnded',
    [RESULT_CODE.DISCOVER_NOT_READY]: 'result.discoverNotReady',
    [RESULT_CODE.DISCOVER_QUEUED_CANCELLED]: 'result.discoverQueuedCancelled',
    [RESULT_CODE.DISCOVER_COLLECT_CONFIG_MISSING]: 'result.discoverCollectConfigMissing',
    [RESULT_CODE.DEVICE_DELETED]: 'result.deviceDeleted',
    [RESULT_CODE.SKILL_TASK_ID_EMPTY]: 'result.skillTaskIdEmpty',
    [RESULT_CODE.SKILL_TASK_RUNNING]: 'result.skillTaskRunning',
    [RESULT_CODE.SKILL_TASK_NOT_FOUND]: 'result.skillTaskNotFound',
    [RESULT_CODE.SKILL_EXTRACT_INPUT_REQUIRED]: 'result.skillExtractInputRequired',
    [RESULT_CODE.PROVIDER_NOT_FOUND]: 'result.providerNotFound',
    [RESULT_CODE.EXEC_TIMEOUT_INVALID]: 'result.execTimeoutInvalid',
    [RESULT_CODE.SKILL_EXTRACT_EMPTY]: 'result.skillExtractEmpty',
    [RESULT_CODE.CLOUD_SYNC_NO_DATA]: 'result.cloudSyncNoData',
    [RESULT_CODE.DEVICE_NOT_FOUND]: 'result.deviceNotFound',
    [RESULT_CODE.JSON_ARRAY_REQUIRED]: 'result.jsonArrayRequired',
    [RESULT_CODE.JSON_OBJECT_REQUIRED]: 'result.jsonObjectRequired',
    [RESULT_CODE.CLIPBOARD_WRITE_FAILED]: 'result.clipboardWriteFailed',
    [RESULT_CODE.HAPTIC_UNSUPPORTED]: 'result.hapticUnsupported',
    [RESULT_CODE.WINDOW_OP_UNSUPPORTED]: 'result.windowOpUnsupported',
    [RESULT_CODE.WINDOW_OP_FAILED]: 'result.windowOpFailed',
    [RESULT_CODE.MAIN_WINDOW_NOT_FOUND]: 'result.mainWindowNotFound',
    [RESULT_CODE.DRAG_UNSUPPORTED]: 'result.dragUnsupported',
    [RESULT_CODE.SESSION_ID_EMPTY]: 'result.sessionIdEmpty',
    [RESULT_CODE.DEVICE_ID_EMPTY]: 'result.deviceIdEmpty',
    [RESULT_CODE.DEVICE_ID_LOCAL]: 'result.deviceIdLocal',
    [RESULT_CODE.SSH_SHELL_NOT_OPENED]: 'result.sshShellNotOpened',
    [RESULT_CODE.SSH_CONFIG_NOT_FOUND]: 'result.sshConfigNotFound',
    [RESULT_CODE.CONFIG_ID_MISMATCH]: 'result.configIdMismatch',
    [RESULT_CODE.SKILL_EXTRACT_PARSE_FAILED]: 'result.skillExtractParseFailed',
    [RESULT_CODE.OPEN_URL_FAILED]: 'result.openUrlFailed',
    [RESULT_CODE.MODEL_MAX_TOKEN_MISSING]: 'result.modelMaxTokenMissing',
    [RESULT_CODE.CLIPBOARD_READ_FAILED]: 'result.clipboardReadFailed',
    [RESULT_CODE.CHAT_IMAGE_NOT_FOUND]: 'result.chatImageNotFound',
    [RESULT_CODE.CHAT_IMAGE_SAVE_FAILED]: 'result.chatImageSaveFailed',
    [RESULT_CODE.STREAM_IDLE_TIMEOUT]: 'result.streamIdleTimeout',
    [RESULT_CODE.LOCAL_PATH_NOT_FOUND]: 'result.localPathNotFound',
    [RESULT_CODE.OPEN_LOCAL_PATH_FAILED]: 'result.openLocalPathFailed',
    [RESULT_CODE.GROUP_DEFAULT_PROTECTED]: 'result.groupDefaultProtected',
};

export interface ResultModel<T> {
    code: number;
    message: string | null;
    data?: T | null;
}

export class BizError extends Error {
    code: number;
    bizMessage: string | null;

    constructor(code: number, message: string | null = null) {
        super(message === null ? undefined : message);
        this.code = code;
        this.bizMessage = message;
    }
}

export function resolveResultMessage(code: number): string {
    const key = RESULT_I18N_KEY[code];
    if (key === undefined) {
        throw new Error(String(code));
    }
    return translate(key);
}

export function resolveLoginErrorPayload(payload: string): string {
    const code = Number(payload);
    if (!Number.isInteger(code)) {
        return payload;
    }
    const key = RESULT_I18N_KEY[code];
    if (key === undefined) {
        return payload;
    }
    return translate(key);
}

export function resolveErrorText(err: unknown): string {
    if (err instanceof BizError) {
        if (err.bizMessage !== null) {
            return err.bizMessage;
        }
        return resolveResultMessage(err.code);
    }
    if (err instanceof Error) {
        return err.message;
    }
    return String(err);
}
