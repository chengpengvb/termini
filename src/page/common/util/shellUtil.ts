import {Icon} from '@iconify/react';
import {Terminal} from '@mui/icons-material';
import {translate} from '@/common/i18n/i18n.ts';
import type {IconifyIcon} from '@iconify/types';
import {createElement, type ReactNode} from 'react';
import iconZsh from '@iconify-icons/simple-icons/zsh';
import iconLinux from '@iconify-icons/simple-icons/linux';
import iconGnuBash from '@iconify-icons/simple-icons/gnubash';
import iconWindows from '@iconify-icons/mdi/microsoft-windows';
import iconPowerShell from '@iconify-icons/simple-icons/powershell';

export const SHELL_KIND = {
    UNKNOWN: 0,
    CMD: 1,
    POWERSHELL: 2,
    PWSH: 3,
    WSL: 4,
    GIT_BASH: 5,
    BASH: 6,
    ZSH: 7,
    FISH: 8,
    DASH: 9,
    SH: 10,
    CSH: 11,
    TCSH: 12,
    KSH: 13,
    BUSYBOX: 14,
} as const;

const SHELL_KIND_ICON_DATA_MAP: Record<number, IconifyIcon | null> = {
    [SHELL_KIND.UNKNOWN]: null,
    [SHELL_KIND.CMD]: iconWindows,
    [SHELL_KIND.POWERSHELL]: iconPowerShell,
    [SHELL_KIND.PWSH]: iconPowerShell,
    [SHELL_KIND.WSL]: iconLinux,
    [SHELL_KIND.GIT_BASH]: iconGnuBash,
    [SHELL_KIND.BASH]: iconGnuBash,
    [SHELL_KIND.ZSH]: iconZsh,
    [SHELL_KIND.FISH]: null,
    [SHELL_KIND.DASH]: null,
    [SHELL_KIND.SH]: null,
    [SHELL_KIND.CSH]: null,
    [SHELL_KIND.TCSH]: null,
    [SHELL_KIND.KSH]: null,
    [SHELL_KIND.BUSYBOX]: null,
};

function extractShellName(id: string): string {
    const dot = id.indexOf('.');
    if (dot < 0) {
        return id;
    }
    return id.substring(dot + 1);
}

function resolveWindowsShellKind(name: string): number {
    if (name === 'cmd') {
        return SHELL_KIND.CMD;
    }
    if (name === 'powershell') {
        return SHELL_KIND.POWERSHELL;
    }
    if (name === 'pwsh') {
        return SHELL_KIND.PWSH;
    }
    if (name === 'wsl') {
        return SHELL_KIND.WSL;
    }
    if (name === 'git_bash') {
        return SHELL_KIND.GIT_BASH;
    }
    return SHELL_KIND.UNKNOWN;
}

function resolveUnixShellKind(name: string): number {
    if (name === 'bash') {
        return SHELL_KIND.BASH;
    }
    if (name === 'zsh') {
        return SHELL_KIND.ZSH;
    }
    if (name === 'fish') {
        return SHELL_KIND.FISH;
    }
    if (name === 'dash') {
        return SHELL_KIND.DASH;
    }
    if (name === 'sh') {
        return SHELL_KIND.SH;
    }
    if (name === 'csh') {
        return SHELL_KIND.CSH;
    }
    if (name === 'tcsh') {
        return SHELL_KIND.TCSH;
    }
    if (name === 'ksh') {
        return SHELL_KIND.KSH;
    }
    if (name === 'busybox') {
        return SHELL_KIND.BUSYBOX;
    }
    return SHELL_KIND.UNKNOWN;
}

export function resolveShellKind(id: string): number {
    const name = extractShellName(id);
    const win = resolveWindowsShellKind(name);
    if (win !== SHELL_KIND.UNKNOWN) {
        return win;
    }
    const unix = resolveUnixShellKind(name);
    if (unix !== SHELL_KIND.UNKNOWN) {
        return unix;
    }
    return SHELL_KIND.UNKNOWN;
}

export function resolveShellLabel(kind: number): string {
    return translate(`common.shell.label.${kind}`);
}

export function resolveShellPromptKind(shellId: string): string {
    return translate(`common.shell.desc.${resolveShellKind(shellId)}`);
}

export function resolveShellIcon(kind: number): ReactNode {
    const iconData = SHELL_KIND_ICON_DATA_MAP[kind];
    if (!iconData) {
        return createElement(Terminal);
    }
    return createElement(Icon, {
        icon: iconData,
        inline: false
    });
}
