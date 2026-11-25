import type {ChangeEvent, MouseEvent} from 'react';
import {Box, Switch, Tooltip, Typography} from '@mui/material';

export function SwitchField(props: { label: string; value: boolean; onChange: (value: boolean) => void; description?: string }) {
    const {label, value, onChange, description} = props;

    const handleChange = (_: ChangeEvent<HTMLInputElement>, checked: boolean): void => {
        onChange(checked);
    };

    const handleRowClick = (): void => {
        onChange(!value);
    };

    const handleSwitchClick = (event: MouseEvent): void => {
        event.stopPropagation();
    };

    const resolveTooltipTitle = (): string | null => {
        if (!description) {
            return null;
        }
        const trimmed = description.trim();
        if (!trimmed) {
            return null;
        }
        return trimmed;
    };

    const tooltipTitle = resolveTooltipTitle();

    return (
        <Box
            className="tm-edit-field"
            onClick={handleRowClick}
            sx={{gap: 1, px: 1, display: 'flex', width: '100%', maxWidth: '100%', minWidth: 0, flexShrink: 0, boxSizing: 'border-box', minHeight: '38px', alignItems: 'center', cursor: 'pointer', border: '1px solid var(--tm-border-subtle)', borderRadius: '7px', bgcolor: 'transparent', transition: 'border-color 120ms ease, background-color 120ms ease', '&:hover': {bgcolor: 'var(--tm-bg-hover)'}, '&:focus-within': {bgcolor: 'var(--tm-bg-hover)', borderColor: 'var(--tm-border-accent)'}}}
        >
            {tooltipTitle ? (
                <Tooltip title={tooltipTitle} enterDelay={1000} enterNextDelay={1000} placement="top-start" arrow>
                    <Typography sx={{flex: 1, minWidth: 0, fontSize: 13, lineHeight: 1.4, color: 'var(--tm-text-secondary)', overflow: 'clip', textOverflow: 'ellipsis', whiteSpace: 'nowrap'}}>
                        {label}
                    </Typography>
                </Tooltip>
            ) : (
                <Typography sx={{flex: 1, minWidth: 0, fontSize: 13, lineHeight: 1.4, color: 'var(--tm-text-secondary)', overflow: 'clip', textOverflow: 'ellipsis', whiteSpace: 'nowrap'}}>
                    {label}
                </Typography>
            )}
            <Switch
                checked={value}
                onChange={handleChange}
                onClick={handleSwitchClick}
                size="small"
                sx={{'& .MuiSwitch-switchBase.Mui-checked': {color: 'var(--tm-status-success)'}, '& .MuiSwitch-switchBase.Mui-checked + .MuiSwitch-track': {backgroundColor: 'var(--tm-status-success)', opacity: 0.45}, '& .MuiSwitch-track': {backgroundColor: 'var(--tm-bg-active)', opacity: 1}}}
            />
        </Box>
    );
}
