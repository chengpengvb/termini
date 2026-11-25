export const DEVICE_CONNECTION_TYPE = {
    LOCAL: 0,
    REMOTE: 1,
} as const;

export const DEVICE_OS_TYPE = {
    UNKNOWN: 0,
    UBUNTU: 101,
    DEBIAN: 102,
    CENTOS: 103,
    RHEL: 104,
    ROCKY: 105,
    ALMALINUX: 106,
    ARCH: 107,
    OPENSUSE: 108,
    FEDORA: 109,
    LINUX: 199,
    MACOS: 201,
    WINDOWS: 301,
    FREEBSD: 401,
    IOS: 501,
    ANDROID: 502,
    CISCO_IOS: 601,
    HUAWEI_VRP: 602,
    H3C_COMWARE: 603,
    RUIJIE_OS: 604,
    ZTE_ZXROS: 605,
    NETWORK_OS: 699,
} as const;

export interface Device {
    id: string;
    name: string;
    host: string;
    type: number;
    os_type: number;
    profile?: string;
    updated_at: number;
}

