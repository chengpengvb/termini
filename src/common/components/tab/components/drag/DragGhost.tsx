import {Box} from '@mui/material';
import type {Ref, ReactNode} from 'react';
import {calculateDragGhostSize, type PaneDragState} from "../../model/PaneDrag.ts";

export function DragGhost(props: { drag: PaneDragState; innerRef: Ref<HTMLDivElement>; }): ReactNode {
    const {drag, innerRef} = props;
    const {width, height} = calculateDragGhostSize(drag);
    const { icon, title } = drag.dragTarget;
    return (
        <Box ref={innerRef} className="tm-drag-ghost" sx={{width, height}}>
            {icon ? (
                <Box className="tm-drag-ghost-icon">{icon}</Box>
            ) : null}
            {title ? <Box className="tm-drag-ghost-title">{title}</Box> : null}
        </Box>
    );
}
