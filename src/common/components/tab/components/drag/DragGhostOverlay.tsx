import {Box} from '@mui/material';
import type {ReactNode} from 'react';
import {DragGhost} from './DragGhost';
import {EVENT_TYPE} from '../../core/types';
import {useRef, useState, useEffect} from 'react';
import type {DropRect} from '../../util/splitDrag';
import {useTabContext} from '../../hooks/useTabContext';
import {PHASE_TYPE, type PaneDragState, calculateDragGhostSize} from '../../model/PaneDrag';

export function DragGhostOverlay(): ReactNode {
    const {manager} = useTabContext();
    const ghostRafRef = useRef<number>(0);
    const dragRef = useRef<PaneDragState | null>(null);
    const ghostElRef = useRef<HTMLDivElement | null>(null);
    const [drag, setDrag] = useState<PaneDragState | null>(null);
    const [landingRect, setLandingRect] = useState<DropRect | null>(null);
    const ghostOffsetRef = useRef<{offsetX: number; offsetY: number}>({offsetX: 0, offsetY: 0});

    const updateGhostTransform = (next: PaneDragState): void => {
        const el = ghostElRef.current;
        if (!el) {
            return;
        }
        const {offsetX, offsetY} = ghostOffsetRef.current;
        const left = next.clientX - offsetX;
        const top = next.clientY - offsetY;
        el.style.transform = `translate3d(${left}px, ${top}px, 0)`;
    };

    const scheduleGhostTransform = (): void => {
        if (ghostRafRef.current) {
            return;
        }
        ghostRafRef.current = requestAnimationFrame(() => {
            ghostRafRef.current = 0;
            const current = dragRef.current;
            if (!current) {
                return;
            }
            updateGhostTransform(current);
        });
    };

    const clearGhost = (): void => {
        setDrag(null);
        dragRef.current = null;
        if (!ghostRafRef.current) {
            return;
        }
        cancelAnimationFrame(ghostRafRef.current);
        ghostRafRef.current = 0;
    };

    const handleDragEvent = (next: PaneDragState): void => {
        dragRef.current = next;
        if (next.phaseType === PHASE_TYPE.END) {
            clearGhost();
            return;
        }
        if (next.phaseType === PHASE_TYPE.START) {
            const {width, height} = calculateDragGhostSize(next);
            ghostOffsetRef.current = {offsetX: width / 2, offsetY: height / 2};
            setDrag(next);
            scheduleGhostTransform();
            return;
        }
        scheduleGhostTransform();
    };

    const handleLandingEnd = (): void => {
        setLandingRect(null);
    };

    useEffect(() => {
        return manager.addEventListener((event: number, data: any) => {
            if (EVENT_TYPE.DRAG === event) {
                handleDragEvent(data as PaneDragState);
            } else if (EVENT_TYPE.DROP_LANDING === event) {
                setLandingRect(data as DropRect);
            }
        });
    }, [manager]);

    return (
        <>
            {drag !== null ? <DragGhost drag={drag} innerRef={ghostElRef}/> : null}
            {landingRect !== null ? (
                <Box className="tm-drop-landing" sx={{left: landingRect.left, top: landingRect.top, width: landingRect.width, height: landingRect.height}} onAnimationEnd={handleLandingEnd}/>
            ) : null}
        </>
    );
}
