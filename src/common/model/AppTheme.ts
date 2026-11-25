export const APP_THEME = {
    DEFAULT: 'default',
    LIGHT: 'light',
} as const;

export const APP_THEME_OPTIONS = [
    {id: APP_THEME.DEFAULT, nameKey: 'set.theme.dark', bg: '#181818', lines: ['#d4d4d4', '#a1a1a1', '#69abe6']},
    {id: APP_THEME.LIGHT, nameKey: 'set.theme.light', bg: '#ffffff', lines: ['#20242b', '#4f5b6b', '#2463b5']},
];
