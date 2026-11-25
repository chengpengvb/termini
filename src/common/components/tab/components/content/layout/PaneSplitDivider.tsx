import { Box } from '@mui/material';
import type { ReactNode, RefObject } from 'react';
import { useEffect, useRef, useState } from 'react';
import { useTabContext } from '../../../hooks/useTabContext';
import { computeClampedRatio } from '../../../util/splitDrag';
import type { PointerEvent as ReactPointerEvent } from 'react';

interface PaneSplitDividerProps {
    splitId: string;
    isHorizontal: boolean;
    containerRef: RefObject<HTMLDivElement | null>;
}

export function PaneSplitDivider(props: PaneSplitDividerProps): ReactNode {
    const { manager } = useTabContext();
    const [isHovering, setIsHovering] = useState(false);
    const [isDragging, setIsDragging] = useState(false);
    const isActive = isHovering || isDragging;
    const { isHorizontal, splitId, containerRef } = props;

    const pointerIdRef = useRef<number | null>(null);

    const handlePointerDown = (e: ReactPointerEvent<HTMLDivElement>): void => {
        if (e.button !== 0) {
            return;
        }
        e.preventDefault();
        pointerIdRef.current = e.pointerId;
        setIsDragging(true);
    };

    const armResizePreview = (): (() => void) => {
        const activePointerId = pointerIdRef.current;
        const bodyClass = isHorizontal ? 'tm-split-resize-col' : 'tm-split-resize-row';
        document.body.classList.add(bodyClass);
        const container = containerRef.current;
        const first = container ? (container.firstElementChild as HTMLElement | null) : null;
        const second = container ? (container.lastElementChild as HTMLElement | null) : null;
        if (!container || !first || !second) {
            return () => {
                document.body.classList.remove(bodyClass);
            };
        }
        const rect = container.getBoundingClientRect();
        const firstFlex = first.style.flex;
        const secondFlex = second.style.flex;
        let didCommit = false;
        const applyRatio = (ratio: number): void => { first.style.flex = String(ratio); second.style.flex = String(1 - ratio); };
        const handleMove = (ev: PointerEvent): void => {
            if (activePointerId !== null && ev.pointerId !== activePointerId) {
                return;
            }
            const ratio = computeClampedRatio(rect, ev.clientX, ev.clientY, isHorizontal);
            if (ratio === null) {
                return;
            }
            applyRatio(ratio);
        };
        const handleUp = (ev: PointerEvent): void => {
            if (activePointerId !== null && ev.pointerId !== activePointerId) {
                return;
            }
            const ratio = computeClampedRatio(rect, ev.clientX, ev.clientY, isHorizontal);
            if (ratio !== null) {
                applyRatio(ratio);
                didCommit = true;
                manager.setSplitRatio(splitId, ratio);
            }
            pointerIdRef.current = null;
            setIsDragging(false);
        };
        window.addEventListener('pointermove', handleMove);
        window.addEventListener('pointerup', handleUp);
        window.addEventListener('pointercancel', handleUp);
        return () => {
            window.removeEventListener('pointermove', handleMove);
            window.removeEventListener('pointerup', handleUp);
            window.removeEventListener('pointercancel', handleUp);
            if (!didCommit) {
                first.style.flex = firstFlex;
                second.style.flex = secondFlex;
            }
            document.body.classList.remove(bodyClass);
        };
    };

    useEffect(() => {
        if (!isDragging) {
            return;
        }
        return armResizePreview();
    }, [containerRef, isDragging, isHorizontal, manager, splitId]);

    return (
        <Box
            onPointerDown={handlePointerDown}
            onMouseEnter={() => {
                setIsHovering(true);
            }}
            onMouseLeave={() => {
                setIsHovering(false);
            }}
            className={isActive ? 'tm-split-divider tm-split-divider-active' : 'tm-split-divider'}
            sx={{ width: isHorizontal ? '6px' : '100%', height: isHorizontal ? '100%' : '6px', cursor: isHorizontal ? 'col-resize' : 'row-resize' }}
        >
            <Box className={isActive ? 'tm-split-handle tm-split-handle-active' : 'tm-split-handle'} sx={{ width: isHorizontal ? '4px' : '32px', height: isHorizontal ? '32px' : '4px' }} />
        </Box>
    );
}
