import type {ReactNode} from 'react';
import {Box, ButtonBase, Typography} from '@mui/material';

export interface NavMenuItemProps {
    label: string;
    icon?: ReactNode;
    title?: string;
    navMenu?: number;
    selected?: boolean;
    className?: string;
    onClick?: () => void;
    endAdornment?: ReactNode;
}

export function NavMenuItem(props: NavMenuItemProps) {
    const {label, icon, selected, endAdornment, className, title, onClick, navMenu} = props;
    const classes = ['tm-nav-item'];
    if (selected) {
        classes.push('is-selected');
    }
    if (className) {
        classes.push(className);
    }

    return (
        <ButtonBase disableRipple className={classes.join(' ')} title={title} onClick={onClick} data-tm-nav-menu={navMenu}>
            {icon != null && (
                <Box className="tm-nav-item-icon-slot" aria-hidden>
                    {icon}
                </Box>
            )}
            <Typography component="span" className="tm-nav-item-label">{label}</Typography>
            {endAdornment}
        </ButtonBase>
    );
}
