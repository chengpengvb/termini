export const HAPTIC_KIND = {
    SELECTION: 'selection',
    IMPACT: 'impact',
    NOTIFICATION: 'notification',
    VIBRATE: 'vibrate',
} as const;

export type HapticKind = typeof HAPTIC_KIND[keyof typeof HAPTIC_KIND];

export const HAPTIC_IMPACT_STYLE = {
    LIGHT: 'light',
    MEDIUM: 'medium',
    HEAVY: 'heavy',
    SOFT: 'soft',
    RIGID: 'rigid',
} as const;

export type HapticImpactStyle = typeof HAPTIC_IMPACT_STYLE[keyof typeof HAPTIC_IMPACT_STYLE];

export const HAPTIC_NOTIFICATION_TYPE = {
    SUCCESS: 'success',
    WARNING: 'warning',
    ERROR: 'error',
} as const;

export type HapticNotificationType = typeof HAPTIC_NOTIFICATION_TYPE[keyof typeof HAPTIC_NOTIFICATION_TYPE];

export interface HapticRequest {
    kind: HapticKind;
    style?: HapticImpactStyle;
    notificationType?: HapticNotificationType;
    duration?: number;
}
