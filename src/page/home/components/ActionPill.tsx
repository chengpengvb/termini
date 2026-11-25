import type {ReactNode} from 'react';
import {Box, ButtonBase} from '@mui/material';

export interface ActionPillProps {
    icon: ReactNode;
    label: string;
    primary?: boolean;
    trailing?: ReactNode;
    onClick?: () => void;
}

export function ActionPill(props: ActionPillProps) {
    const {icon, label, primary, trailing, onClick} = props;

    const handleClick = (): void => {
        if (!onClick) {
            return;
        }
        onClick();
    };

    const className = primary === true ? 'tm-action-btn tm-action-btn-primary' : 'tm-action-btn';

    return (
        <ButtonBase onClick={handleClick} className={className}>
            {icon}
            <Box component="span" className="tm-action-btn-label">
                {label}
            </Box>
            {trailing ? trailing : null}
        </ButtonBase>
    );
}

