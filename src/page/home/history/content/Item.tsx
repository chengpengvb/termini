import type {ReactNode} from 'react';
import {Box, ButtonBase, Typography} from '@mui/material';

export interface HistoryItemProps {
    title: string;
    meta?: string;
    icon: ReactNode;
    selected?: boolean;
    onClick: () => void;
}

export function Item(props: HistoryItemProps) {
    const {title, meta, icon, selected, onClick} = props;

    return (
        <ButtonBase disableRipple className={selected ? 'tm-history-item is-selected' : 'tm-history-item'} onClick={onClick}>
            <Box className="tm-history-item-icon" aria-hidden>
                {icon}
            </Box>
            <Typography className="tm-history-item-title tm-ellipsis">{title}</Typography>
            {meta ? <Typography className="tm-history-item-meta">{meta}</Typography> : null}
        </ButtonBase>
    );
}
