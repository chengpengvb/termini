import { Box } from '@mui/material';
import type { ReactNode } from 'react';
import type { DropRect } from '../../../util/splitDrag';

interface PaneDropOverlayProps {
    rect: DropRect | null;
}

export function PaneDropOverlay(props: PaneDropOverlayProps): ReactNode {
    const { rect } = props;
    if (!rect) {
        return null;
    }
    return (
        <Box className="tm-drop-overlay" sx={{ left: rect.left, top: rect.top, width: rect.width, height: rect.height }} />
    );
}
