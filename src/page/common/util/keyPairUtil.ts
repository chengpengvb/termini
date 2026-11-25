import type {CSSProperties} from 'react';
import {translate} from '@/common/i18n/i18n.ts';
import {ConfigKeyPair} from '@/common/model/ConfigKeyPair.ts';
import {isEmpty, isNotEmpty} from '@/common/util/StringUtil.ts';
import {resolveContrastForeground} from '@/page/common/util/colorContrastUtil.ts';

const KEY_PAIR_AVATAR_COLOR_PALETTE = [
    '#3a3a3c',
    '#0a84ff',
    '#2c2c2e',
    '#636366',
    '#48484a',
    '#1c1c1e',
    '#8e8e93',
    '#64d2ff',
] as const;

export function resolveKeyPairAvatarLetter(name: string | undefined): string {
    if (isEmpty(name)) {
        return '';
    }
    return name.trim().charAt(0).toUpperCase();
}

export function resolveKeyPairAvatarColor(name: string | undefined): string {
    const letter = resolveKeyPairAvatarLetter(name);
    if (isEmpty(letter)) {
        throw new Error('key pair avatar color requires a non-empty name');
    }
    const codePoint = letter.codePointAt(0);
    if (codePoint === undefined) {
        throw new Error('key pair avatar color requires a valid first character');
    }
    const index = (Math.imul(codePoint, 2654435761) >>> 0) % KEY_PAIR_AVATAR_COLOR_PALETTE.length;
    return KEY_PAIR_AVATAR_COLOR_PALETTE[index];
}

export function resolveKeyPairAvatarStyle(name: string | undefined): CSSProperties {
    const color = resolveKeyPairAvatarColor(name);
    return {
        '--tm-device-os-color': color,
        '--tm-icon-on-brand': resolveContrastForeground(color),
    } as CSSProperties;
}

export function resolveKeyPairStatusText(value: ConfigKeyPair): string {
    const hasPublic = isNotEmpty(value.public_key);
    const hasPrivate = isNotEmpty(value.private_key);
    if (!hasPublic && !hasPrivate) {
        return translate('home.keys.status.unconfigured');
    }
    if (hasPublic && hasPrivate) {
        return translate('home.keys.status.both');
    }
    if (hasPrivate) {
        return translate('home.keys.status.privateOnly');
    }
    return translate('home.keys.status.publicOnly');
}
