export type RdpMobileQuality = 'smooth' | 'balanced' | 'sharp';
export interface RdpMobilePreferences {
    touchMode: boolean;
    quality: RdpMobileQuality;
}

export const RDP_MOBILE_QUALITIES: RdpMobileQuality[] = ['smooth', 'balanced', 'sharp'];
const PROFILES = {
    smooth: {edge: 1920, pixels: 2_100_000},
    balanced: {edge: 2560, pixels: 4_000_000},
    sharp: {edge: 3840, pixels: 8_000_000},
};

export function readRdpMobilePreferences(deviceId = 'default'): RdpMobilePreferences {
    const defaults: RdpMobilePreferences = {touchMode: true, quality: 'balanced'};
    try {
        const value = localStorage.getItem('tm-rdp-mobile:' + deviceId);
        if (!value) {
            return defaults;
        }
        const data = JSON.parse(value) as Partial<RdpMobilePreferences>;
        return {
            touchMode: typeof data.touchMode === 'boolean' ? data.touchMode : defaults.touchMode,
            quality: data.quality && RDP_MOBILE_QUALITIES.includes(data.quality) ? data.quality : defaults.quality,
        };
    } catch (error) {
        console.warn('[rdp] mobile preference read failed', error);
        return defaults;
    }
}

export function saveRdpMobilePreferences(deviceId: string | undefined, value: RdpMobilePreferences): void {
    try {
        localStorage.setItem('tm-rdp-mobile:' + (deviceId ?? 'default'), JSON.stringify(value));
    } catch (error) {
        console.warn('[rdp] mobile preference save failed', error);
    }
}

export function mobileRdpPixels(width: number, height: number, quality: RdpMobileQuality) {
    if (!Number.isFinite(width) || !Number.isFinite(height) || width <= 0 || height <= 0) {
        throw new Error('Invalid mobile RDP screen dimensions');
    }
    const profile = PROFILES[quality];
    const scale = Math.min(1, profile.edge / Math.max(width, height), Math.sqrt(profile.pixels / (width * height)));
    return {width: Math.max(2, Math.floor(width * scale / 2) * 2), height: Math.max(2, Math.floor(height * scale / 2) * 2)};
}
