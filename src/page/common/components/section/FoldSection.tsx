import { useState } from 'react';
import type { ReactNode } from 'react';
import { ExpandMore } from '@mui/icons-material';
import { Box, ButtonBase, Collapse, Typography } from '@mui/material';

export function FoldSection(props: { label: string; children: ReactNode }) {
    const { label, children } = props;
    const [open, setOpen] = useState<boolean>(false);

    const handleToggle = (): void => {
        setOpen(!open);
    };

    return (
        <>
            <ButtonBase onClick={handleToggle} sx={{ display: 'flex', alignItems: 'center', width: '100%', flexShrink: 0, minHeight: '38px', px: 0, color: 'var(--tm-text-tertiary)', '&:hover': { color: 'var(--tm-text-primary)' } }}>
                <Typography sx={{ flex: 1, minWidth: 0, pl: 1, textAlign: 'left', fontSize: 13, fontWeight: 500 }}>{label}</Typography>
                <Box sx={{ ml: 'auto', width: 26, height: 26, display: 'flex', alignItems: 'center', justifyContent: 'center', borderRadius: '8px', mr: '6px', '&:hover': { bgcolor: 'var(--tm-bg-selected)', color: 'var(--tm-text-primary)' } }}>
                    <ExpandMore sx={{ fontSize: 16, transform: open ? 'rotate(180deg)' : 'rotate(0deg)', transition: 'transform var(--tm-duration-page) var(--tm-easing-ios)' }} />
                </Box>
            </ButtonBase>
            <Collapse in={open} timeout={280} mountOnEnter unmountOnExit sx={{ flexShrink: 0, transitionTimingFunction: 'var(--tm-easing-ios)' }}>
                <Box sx={{ display: 'flex', flexDirection: 'column', gap: 1 }}>{children}</Box>
            </Collapse>
        </>
    );
}
