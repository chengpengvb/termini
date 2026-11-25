import {Check} from '@mui/icons-material';
import {Box, ButtonBase, Typography} from '@mui/material';
import {resolveTerminalThemeSubtitle, TERMINAL_THEME_OPTIONS, TerminalThemeOption} from '@/page/common/util/terminal.ts';

const SWATCH_LINE_WIDTHS = ['70%', '90%', '55%'];

function ThemeSwatch(props: { theme: TerminalThemeOption }) {
    const {theme} = props;
    return (
        <Box className="tm-theme-item-swatch" sx={{backgroundColor: theme.bg}}>
            {theme.lines.map((color, index) => (
                <Box key={index} className="tm-theme-item-swatch-line" sx={{backgroundColor: color, width: SWATCH_LINE_WIDTHS[index] ?? '60%'}}/>
            ))}
        </Box>
    );
}

function ThemeItem(props: { theme: TerminalThemeOption; selected: boolean; subtitle: string; onSelect: () => void }) {
    const {theme, selected, subtitle, onSelect} = props;
    const className = selected ? 'tm-device-item tm-theme-item is-selected' : 'tm-device-item tm-theme-item';

    return (
        <ButtonBase className={className} aria-pressed={selected} title={theme.name} onClick={onSelect}>
            <ThemeSwatch theme={theme}/>
            <Box className="tm-device-item-info">
                <Typography className="tm-device-item-name">{theme.name}</Typography>
                <Typography className="tm-device-item-ip">{subtitle}</Typography>
            </Box>
            {selected ? <Check aria-hidden="true" sx={{fontSize: 18, color: 'var(--tm-icon-accent)', flexShrink: 0}}/> : null}
        </ButtonBase>
    );
}

export function ThemeField(props: { value: string; onChange: (value: string) => void; themes?: TerminalThemeOption[]; compact?: boolean }) {
    const {value, onChange, themes = TERMINAL_THEME_OPTIONS, compact = false} = props;

    return (
        <Box className={compact ? 'tm-theme-item-list is-compact' : 'tm-theme-item-list'}>
            {themes.map(theme => (
                <ThemeItem key={theme.id} theme={theme} selected={theme.id === value} subtitle={resolveTerminalThemeSubtitle(theme)} onSelect={() => onChange(theme.id)}/>
            ))}
        </Box>
    );
}
