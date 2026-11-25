import type {ReactNode} from 'react';
import {Box, Typography} from '@mui/material';

export function Section(props: { title: string; children: ReactNode }) {
    const {title, children} = props;

    return (
        <Box sx={{display: 'flex', flexDirection: 'column', flexShrink: 0}}>
            <Typography sx={{fontSize: 13, fontWeight: 600, color: 'var(--tm-text-primary)', letterSpacing: 0.2, mb: 1.25}}>{title}</Typography>
            <Box sx={{display: 'flex', flexDirection: 'column', gap: 1}}>
                {children}
            </Box>
        </Box>
    );
}
