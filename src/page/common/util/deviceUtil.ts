import {Icon} from "@iconify/react";
import iconCisco from "@/assets/icon/cisco";
import {Computer} from "@mui/icons-material";
import {translate} from '@/common/i18n/i18n.ts';
import type {IconifyIcon} from "@iconify/types";
import {PROTOCOL} from "@/common/const/Const.ts";
import {ReqDevice} from "@/common/model/req/ReqDevice";
import {DEVICE_OS_TYPE} from "@/common/model/Device.ts";
import {isEmpty, isNil} from "@/common/util/StringUtil";
import iconApple from "@iconify-icons/simple-icons/apple";
import iconLinux from "@iconify-icons/simple-icons/linux";
import iconCentos from "@iconify-icons/simple-icons/centos";
import iconDebian from "@iconify-icons/simple-icons/debian";
import iconFedora from "@iconify-icons/simple-icons/fedora";
import iconHuawei from "@iconify-icons/simple-icons/huawei";
import iconRedhat from "@iconify-icons/simple-icons/redhat";
import iconUbuntu from "@iconify-icons/simple-icons/ubuntu";
import iconAndroid from "@iconify-icons/simple-icons/android";
import iconFreebsd from "@iconify-icons/simple-icons/freebsd";
import iconWindows from "@iconify-icons/mdi/microsoft-windows";
import iconOpensuse from "@iconify-icons/simple-icons/opensuse";
import iconArchlinux from "@iconify-icons/simple-icons/archlinux";
import iconRockylinux from "@iconify-icons/simple-icons/rockylinux";
import {createElement, type CSSProperties, type ReactNode} from "react";
import type {SelectOption} from "@/page/common/components/input/SelectField";
import {resolveContrastForeground} from '@/page/common/util/colorContrastUtil.ts';
import {iconH3c, iconNetworkOs, iconRuijie, iconUnknownOs, iconZte} from "@/assets/icon/deviceOs";

const NAME_MAX_LENGTH = 64;
const HOST_MAX_LENGTH = 253;

const DEVICE_OS_TYPE_ICON_DATA_MAP: Record<number, IconifyIcon | null> = {
    [DEVICE_OS_TYPE.UNKNOWN]: iconUnknownOs,
    [DEVICE_OS_TYPE.UBUNTU]: iconUbuntu,
    [DEVICE_OS_TYPE.DEBIAN]: iconDebian,
    [DEVICE_OS_TYPE.CENTOS]: iconCentos,
    [DEVICE_OS_TYPE.RHEL]: iconRedhat,
    [DEVICE_OS_TYPE.ROCKY]: iconRockylinux,
    [DEVICE_OS_TYPE.ALMALINUX]: iconLinux,
    [DEVICE_OS_TYPE.ARCH]: iconArchlinux,
    [DEVICE_OS_TYPE.OPENSUSE]: iconOpensuse,
    [DEVICE_OS_TYPE.FEDORA]: iconFedora,
    [DEVICE_OS_TYPE.LINUX]: iconLinux,
    [DEVICE_OS_TYPE.MACOS]: iconApple,
    [DEVICE_OS_TYPE.WINDOWS]: iconWindows,
    [DEVICE_OS_TYPE.FREEBSD]: iconFreebsd,
    [DEVICE_OS_TYPE.IOS]: iconApple,
    [DEVICE_OS_TYPE.ANDROID]: iconAndroid,
    [DEVICE_OS_TYPE.CISCO_IOS]: iconCisco,
    [DEVICE_OS_TYPE.HUAWEI_VRP]: iconHuawei,
    [DEVICE_OS_TYPE.H3C_COMWARE]: iconH3c,
    [DEVICE_OS_TYPE.RUIJIE_OS]: iconRuijie,
    [DEVICE_OS_TYPE.ZTE_ZXROS]: iconZte,
    [DEVICE_OS_TYPE.NETWORK_OS]: iconNetworkOs,
};

const DEVICE_OS_TYPE_ICON_COLOR_MAP: Record<number, string> = {
    [DEVICE_OS_TYPE.UNKNOWN]: '#64748B',
    [DEVICE_OS_TYPE.UBUNTU]: '#E95420',
    [DEVICE_OS_TYPE.DEBIAN]: '#A80030',
    [DEVICE_OS_TYPE.CENTOS]: '#262577',
    [DEVICE_OS_TYPE.RHEL]: '#EE0000',
    [DEVICE_OS_TYPE.ROCKY]: '#10B981',
    [DEVICE_OS_TYPE.ALMALINUX]: '#0F6CBD',
    [DEVICE_OS_TYPE.ARCH]: '#1793D1',
    [DEVICE_OS_TYPE.OPENSUSE]: '#73BA25',
    [DEVICE_OS_TYPE.FEDORA]: '#51A2DA',
    [DEVICE_OS_TYPE.LINUX]: '#F0A30A',
    [DEVICE_OS_TYPE.MACOS]: '#636366',
    [DEVICE_OS_TYPE.WINDOWS]: '#0078D4',
    [DEVICE_OS_TYPE.FREEBSD]: '#AB2B28',
    [DEVICE_OS_TYPE.IOS]: '#636366',
    [DEVICE_OS_TYPE.ANDROID]: '#3DDC84',
    [DEVICE_OS_TYPE.CISCO_IOS]: '#049FD9',
    [DEVICE_OS_TYPE.HUAWEI_VRP]: '#CF0A2C',
    [DEVICE_OS_TYPE.H3C_COMWARE]: '#00A651',
    [DEVICE_OS_TYPE.RUIJIE_OS]: '#E31C23',
    [DEVICE_OS_TYPE.ZTE_ZXROS]: '#0057B8',
    [DEVICE_OS_TYPE.NETWORK_OS]: '#0F766E',
};

const DEVICE_OS_TYPE_ICON_WEIGHT_MAP: Record<number, 'thin' | 'dense'> = {
    [DEVICE_OS_TYPE.HUAWEI_VRP]: 'dense',
    [DEVICE_OS_TYPE.H3C_COMWARE]: 'dense',
    [DEVICE_OS_TYPE.ZTE_ZXROS]: 'dense',
};

const PROTOCOL_BADGE_STROKE = 'fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round"';

const PROTOCOL_BADGE_MAP: Record<number, { icon: IconifyIcon; modifier: string }> = {
    [PROTOCOL.SSH]: {
        icon: {width: 24, height: 24, body: `<path d="M9 5.5l6.5 6.5L9 18.5" ${PROTOCOL_BADGE_STROKE} stroke-width="3.6"/>`},
        modifier: 'is-ssh',
    },
    [PROTOCOL.TELNET]: {
        icon: {width: 24, height: 24, body: `<g ${PROTOCOL_BADGE_STROKE} stroke-width="3.6"><path d="M4.5 6h15"/><path d="M12 6v12"/></g>`},
        modifier: 'is-telnet',
    },
    [PROTOCOL.SFTP]: {
        icon: {width: 24, height: 24, body: `<g ${PROTOCOL_BADGE_STROKE} stroke-width="3"><path d="M7.5 19.5V5"/><path d="M3 9l4.5-4.5L12 9"/><path d="M16.5 4.5V19"/><path d="M21 15l-4.5 4.5L12 15"/></g>`},
        modifier: 'is-sftp',
    },
    [PROTOCOL.FTP]: {
        icon: {width: 24, height: 24, body: `<g ${PROTOCOL_BADGE_STROKE} stroke-width="3"><path d="M7.5 19.5V5"/><path d="M3 9l4.5-4.5L12 9"/><path d="M16.5 4.5V19"/><path d="M21 15l-4.5 4.5L12 15"/></g>`},
        modifier: 'is-ftp',
    },
    [PROTOCOL.RDP]: {
        icon: {width: 24, height: 24, body: `<g ${PROTOCOL_BADGE_STROKE} stroke-width="3"><rect x="3" y="4.5" width="18" height="13" rx="1.5"/><path d="M8.5 21h7"/></g>`},
        modifier: 'is-rdp',
    },
    [PROTOCOL.SERIAL]: {
        icon: {width: 24, height: 24, body: `<g ${PROTOCOL_BADGE_STROKE} stroke-width="3"><path d="M8 3v4"/><path d="M16 3v4"/><path d="M5 7h14v4a7 7 0 0 1-14 0z"/><path d="M12 18v3"/></g>`},
        modifier: 'is-serial',
    },
};

const DEVICE_OS_TYPE_IDS: number[] = [
    DEVICE_OS_TYPE.UNKNOWN,
    DEVICE_OS_TYPE.UBUNTU,
    DEVICE_OS_TYPE.DEBIAN,
    DEVICE_OS_TYPE.CENTOS,
    DEVICE_OS_TYPE.RHEL,
    DEVICE_OS_TYPE.ROCKY,
    DEVICE_OS_TYPE.ALMALINUX,
    DEVICE_OS_TYPE.ARCH,
    DEVICE_OS_TYPE.OPENSUSE,
    DEVICE_OS_TYPE.FEDORA,
    DEVICE_OS_TYPE.LINUX,
    DEVICE_OS_TYPE.MACOS,
    DEVICE_OS_TYPE.WINDOWS,
    DEVICE_OS_TYPE.FREEBSD,
    DEVICE_OS_TYPE.IOS,
    DEVICE_OS_TYPE.ANDROID,
    DEVICE_OS_TYPE.CISCO_IOS,
    DEVICE_OS_TYPE.HUAWEI_VRP,
    DEVICE_OS_TYPE.H3C_COMWARE,
    DEVICE_OS_TYPE.RUIJIE_OS,
    DEVICE_OS_TYPE.ZTE_ZXROS,
    DEVICE_OS_TYPE.NETWORK_OS,
];

export const DEVICE_OS_TYPE_OPTIONS: SelectOption[] = DEVICE_OS_TYPE_IDS.map(value => ({
    value: String(value),
    label: translate(`home.devices.osType.${value}`),
}));

export function resolveDeviceOsLabel(osType: number): string {
    if (!DEVICE_OS_TYPE_IDS.includes(osType)) {
        return translate(`home.devices.osType.${DEVICE_OS_TYPE.UNKNOWN}`);
    }
    return translate(`home.devices.osType.${osType}`);
}

export function resolveDeviceOsIconName(osType: number): IconifyIcon | null {
    const iconData = DEVICE_OS_TYPE_ICON_DATA_MAP[osType];
    if (!iconData) {
        return null;
    }
    return iconData;
}

export function resolveDeviceOsIconColor(osType: number): string | null {
    const color = DEVICE_OS_TYPE_ICON_COLOR_MAP[osType];
    if (isNil(color)) {
        return null;
    }
    return color;
}

export function resolveDeviceOsIconChipClassName(osType: number): string {
    const classes = ['tm-device-item-icon'];
    if (resolveDeviceOsIconColor(osType)) {
        classes.push('has-accent');
    }
    const weight = DEVICE_OS_TYPE_ICON_WEIGHT_MAP[osType];
    if (weight === 'thin') {
        classes.push('is-thin');
    }
    if (weight === 'dense') {
        classes.push('is-dense');
    }
    return classes.join(' ');
}

export function resolveDeviceOsIconChipStyle(osType: number): CSSProperties | undefined {
    const color = resolveDeviceOsIconColor(osType);
    if (!color) {
        return undefined;
    }
    return {
        '--tm-device-os-color': color,
        '--tm-icon-on-brand': resolveContrastForeground(color),
        '--tm-device-os-ink': `color-mix(in srgb, ${color} 55%, var(--tm-text-primary))`,
    } as CSSProperties;
}

function resolveProtocolBadge(protocol?: number): ReactNode {
    if (isNil(protocol)) {
        return null;
    }
    const badge = PROTOCOL_BADGE_MAP[protocol];
    if (!badge) {
        return null;
    }
    return createElement(Icon, {icon: badge.icon, className: `tm-proto-badge ${badge.modifier}`, inline: false});
}

export function resolveDeviceOsIcon(osType: number, className: string): ReactNode {
    const iconData = resolveDeviceOsIconName(osType);
    if (!iconData) {
        return createElement(Computer, {className});
    }
    return createElement(Icon, {icon: iconData, className, inline: false});
}

export function resolveDeviceOsIconBadged(osType: number, className: string, protocol?: number, tiled = true): ReactNode {
    const icon = resolveDeviceOsIcon(osType, className);
    const badge = resolveProtocolBadge(protocol);
    const hasTile = tiled === true && !!resolveDeviceOsIconColor(osType);
    const wrapClassName = hasTile ? 'tm-os-icon-badged has-accent' : 'tm-os-icon-badged';
    const wrapStyle = hasTile ? resolveDeviceOsIconChipStyle(osType) : undefined;
    if (!badge) {
        if (!hasTile) {
            return icon;
        }
        return createElement('span', {className: wrapClassName, style: wrapStyle}, icon);
    }
    return createElement('span', {className: wrapClassName, style: wrapStyle}, icon, badge);
}

export function resolveDeviceTabIcon(osType: number, protocol?: number): ReactNode {
    return resolveDeviceOsIconBadged(osType, 'tm-tab-device-os-icon', protocol);
}

function isAllowedDevicePort(port: number): boolean {
    if (port === 0) {
        return true;
    }
    return Number.isInteger(port) && port > 0 && port <= 65535;
}

function resolveUsernameError(username: string | null | undefined): string | null {
    if (isNil(username) || isEmpty(username)) {
        return null;
    }
    if (username.length > NAME_MAX_LENGTH) {
        return translate('common.validation.usernameTooLong');
    }
    return null;
}

export function resolveDeviceError(reqDevice: ReqDevice, devices: ReqDevice[]): string | null {
    const name = reqDevice.device.name;
    if (!isEmpty(name)) {
        if (name.length > NAME_MAX_LENGTH) {
            return translate('common.validation.nameTooLong');
        }
        const conflict = devices.some(item => item.device.name === name && item.device.id !== reqDevice.device.id);
        if (conflict) {
            return translate('common.validation.nameExists');
        }
    }
    const host = reqDevice.device.host;
    if (!isEmpty(host)) {
        if (host.length > HOST_MAX_LENGTH) {
            return translate('common.validation.hostTooLong');
        }
        if (/\s/.test(host)) {
            return translate('common.validation.hostNoSpace');
        }
    }
    const sshPort = reqDevice.config_ssh?.base?.port;
    if (!isNil(sshPort) && !isAllowedDevicePort(sshPort)) {
        return translate('common.validation.portRange');
    }
    const telnetPort = reqDevice.config_telnet?.base?.port;
    if (!isNil(telnetPort) && !isAllowedDevicePort(telnetPort)) {
        return translate('common.validation.portRange');
    }
    const rdpPort = reqDevice.config_rdp?.base?.port;
    if (!isNil(rdpPort) && !isAllowedDevicePort(rdpPort)) {
        return translate('common.validation.portRange');
    }
    const ftpPort = reqDevice.config_ftp?.base?.port;
    if (!isNil(ftpPort) && !isAllowedDevicePort(ftpPort)) {
        return translate('common.validation.portRange');
    }
    const sshUserError = resolveUsernameError(reqDevice.config_ssh?.base?.username);
    if (sshUserError !== null) {
        return sshUserError;
    }
    const telnetUserError = resolveUsernameError(reqDevice.config_telnet?.base?.username);
    if (telnetUserError !== null) {
        return telnetUserError;
    }
    const rdpUserError = resolveUsernameError(reqDevice.config_rdp?.base?.username);
    if (rdpUserError !== null) {
        return rdpUserError;
    }
    const ftpUserError = resolveUsernameError(reqDevice.config_ftp?.base?.username);
    if (ftpUserError !== null) {
        return ftpUserError;
    }
    return null;
}

export function verifyInfo(reqDevice: ReqDevice, devices: ReqDevice[]): boolean {
    if (isEmpty(reqDevice.device.name)) {
        return false;
    }
    if (isEmpty(reqDevice.device.host)) {
        return false;
    }
    if (isEmpty(reqDevice.group_id)) {
        return false;
    }
    return resolveDeviceError(reqDevice, devices) === null;
}
