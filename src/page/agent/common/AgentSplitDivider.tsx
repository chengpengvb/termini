import {Box} from '@mui/material';
import {useEffect, useRef, useState} from 'react';
import {isNil, isNotNil} from '@/common/util/StringUtil.ts';
import type {PointerEvent as ReactPointerEvent, RefObject} from 'react';

export const SPLIT_SIDE = {
    LEFT: 1,
    RIGHT: 2,
} as const;

export type SplitSide = typeof SPLIT_SIDE[keyof typeof SPLIT_SIDE];

interface AgentSplitDividerProps {
    side?: SplitSide;
    minSize?: number;
    minMain?: number;
    onWidthChange: (width: number) => void;
    containerRef: RefObject<HTMLDivElement | null>;
    widthTargetRef?: RefObject<HTMLElement | null>;
}

export function AgentSplitDivider(props: AgentSplitDividerProps) {
    const {containerRef, onWidthChange, side = SPLIT_SIDE.LEFT, minSize = 180, minMain = 320, widthTargetRef} = props;
    const [isHovering, setIsHovering] = useState(false);
    const [isDragging, setIsDragging] = useState(false);
    const pointerIdRef = useRef<number | null>(null);
    const onWidthChangeRef = useRef(onWidthChange);
    onWidthChangeRef.current = onWidthChange;
    const widthTargetRefLocal = useRef(widthTargetRef);
    widthTargetRefLocal.current = widthTargetRef;

    const applyLiveWidth = (width: number): void => {
        const target = widthTargetRefLocal.current?.current;
        if (target) {
            target.style.width = `${width}px`;
            return;
        }
        onWidthChangeRef.current(width);
    };

    const resolveClampedWidth = (clientX: number): number | null => {
        const container = containerRef.current;
        if (!container) {
            return null;
        }
        const rect = container.getBoundingClientRect();
        const maxSize = rect.width - minMain;
        if (maxSize < minSize) {
            return null;
        }
        const raw = side === SPLIT_SIDE.RIGHT ? rect.right - clientX : clientX - rect.left;
        if (raw < minSize) {
            return minSize;
        }
        if (raw > maxSize) {
            return maxSize;
        }
        return raw;
    };

    const handlePointerDown = (e: ReactPointerEvent<HTMLDivElement>): void => {
        if (e.button !== 0) {
            return;
        }
        e.preventDefault();
        pointerIdRef.current = e.pointerId;
        setIsDragging(true);
    };

    const handleMouseEnter = (): void => {
        setIsHovering(true);
    };

    const handleMouseLeave = (): void => {
        setIsHovering(false);
    };

    useEffect(() => {
        if (!isDragging) {
            return;
        }
        const activePointerId = pointerIdRef.current;
        document.body.classList.add('tm-split-resize-col');
        const handleMove = (ev: PointerEvent): void => {
            if (isNotNil(activePointerId) && ev.pointerId !== activePointerId) {
                return;
            }
            const width = resolveClampedWidth(ev.clientX);
            if (isNil(width)) {
                return;
            }
            applyLiveWidth(width);
        };
        const handleUp = (ev: PointerEvent): void => {
            if (isNotNil(activePointerId) && ev.pointerId !== activePointerId) {
                return;
            }
            const width = resolveClampedWidth(ev.clientX);
            if (isNotNil(width)) {
                const target = widthTargetRefLocal.current?.current;
                if (target) {
                    target.style.width = `${width}px`;
                }
                onWidthChangeRef.current(width);
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
            document.body.classList.remove('tm-split-resize-col');
        };
    }, [containerRef, isDragging, side, minSize, minMain]);

    return (
        <Box className={isHovering || isDragging ? 'tm-split-divider tm-split-divider-active tm-agent-split-divider' : 'tm-split-divider tm-agent-split-divider'} onPointerDown={handlePointerDown} onMouseEnter={handleMouseEnter} onMouseLeave={handleMouseLeave}>
            <Box className={isHovering || isDragging ? 'tm-split-handle tm-split-handle-active tm-agent-split-handle' : 'tm-split-handle tm-agent-split-handle'}/>
        </Box>
    );
}
