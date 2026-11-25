export const REQ_WS_EVENT_SHELL = {
    OPEN: 0,
    CLOSE: 1,
    WRITE: 2,
    RESIZE: 3,
    SNAPSHOT: 4,
} as const;

export const REQ_WS_EVENT_TELNET = {
    OPEN: 10,
    CLOSE: 11,
    WRITE: 12,
    RESIZE: 13,
} as const;

export const REQ_WS_EVENT_SSH = {
    OPEN: 20,
    CLOSE: 21,
    WRITE: 22,
    RESIZE: 23,
} as const;

export const REQ_WS_EVENT_SSH_AUTH = {
    AUTH_ANSWER: 30,
} as const;

export const REQ_WS_EVENT_SFTP = {
    OPEN: 40,
    CLOSE: 41,
    HOME_DIR: 42,
    LIST_DIR: 43, // 列出目录内容，返回 entries（含 hasParent/parentPath，有父目录时首项为 ..）
    STAT: 44, // 获取单个路径元信息（name/path/kind/size/mtime/permissions）
    RENAME: 45, // 重命名或移动（from -> to）
    CHMOD: 46, // 修改文件权限（mode）
    CREATE: 47, // 创建文件/目录/符号链接（kind + 可选 linkTarget）
    DELETE: 48, // 删除文件或目录（递归），推送删除进度
    TRANSFER: 49, // 跨客户端传输（源读 + 目标写），推送传输进度
    STAT_PROPERTIES: 50, // 获取详细属性（entry + 目录统计：文件数/总大小等）
    TRANSFER_ANSWER: 51, // 传输过程中文件冲突确认应答
    FILE_OPEN: 52, // 打开文件（远端先下载到临时目录，本地直接打开）
    EXPORT_TO_TEMP: 53, // 将远程文件导出到本地临时目录（用于拖出到 OS）
    TRANSFER_CANCEL: 54, // 取消进行中的传输（按 respId）
    DELETE_CANCEL: 55, // 取消进行中的删除（按 respId）
} as const;

export const REQ_WS_EVENT_RDP = {
    OPEN: 80,
    CLOSE: 81,
    INPUT: 82,
    RESIZE: 83,
    VISIBILITY: 84,
    AUTH_ANSWER: 85,
    FRAME_SYNC: 86,
    FOCUS: 87,
} as const;

export const REQ_WS_EVENT_AGENT = {
    CHAT: 60,
    CANCEL: 61,
    TOOL_ANSWER: 62,
    TOOL_CANCEL: 63,
    SUBSCRIBE: 64,
    UNSUBSCRIBE: 65,
    CONFIRM_MODE: 66,
    QUESTION_ANSWER: 67,
} as const;

export const REQ_WS_EVENT_SERIAL = {
    OPEN: 90,
    CLOSE: 91,
    WRITE: 92,
} as const;