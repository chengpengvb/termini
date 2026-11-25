import {Box} from '@mui/material';
import type {ReactNode} from 'react';
import {EVENT_TYPE} from '../core/types';
import {useEffect, useRef, useState} from 'react';
import {useTabContext} from '../hooks/useTabContext';

export function SidePanel() {
    const { manager } = useTabContext();
    const startXRef = useRef<number>(0);
    const startRatioRef = useRef<number>(0.35);
    const [ratio, setRatio] = useState<number>(0.35);
    const [isDragging, setIsDragging] = useState<boolean>(false);
    const [isHovering, setIsHovering] = useState<boolean>(false);
    const [content, setContent] = useState<ReactNode | null>(() => manager.getSidePanel());
    const [visible, setVisible] = useState<boolean>(() => manager.isSidePanelVisible());

    const handlePointerDown = (e: React.PointerEvent<HTMLDivElement>): void => {
        if (e.button !== 0) {
            return;
        }
        e.preventDefault();
        startXRef.current = e.clientX;
        startRatioRef.current = ratio;
        setIsDragging(true);
    };

    const handleMouseEnter = (): void => {
        setIsHovering(true);
    };

    const handleMouseLeave = (): void => {
        setIsHovering(false);
    };

    const resolveHandleClass = (): string => {
        if (isHovering || isDragging) {
            return 'tm-split-handle tm-split-handle-active';
        }
        return 'tm-split-handle';
    };

    useEffect(() => {
        return manager.addEventListener((event, data) => {
            if (event === EVENT_TYPE.SIDE_PANEL) {
                setContent(data as ReactNode | null);
                return;
            }
            if (event === EVENT_TYPE.SIDE_PANEL_VISIBLE) {
                setVisible(data as boolean);
            }
        });
    }, [manager]);

    useEffect(() => {
        if (!isDragging) {
            return;
        }
        document.body.classList.add('tm-split-resize-col');
        const handleMove = (e: PointerEvent): void => {
            const delta = startXRef.current - e.clientX;
            const next = startRatioRef.current + delta / window.innerWidth;
            setRatio(Math.max(0.15, Math.min(0.5, next)));
        };
        const handleUp = (): void => {
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
    }, [isDragging]);

    if (!content) {
        return null;
    }

    return (
        <Box sx={{ display: visible ? 'flex' : 'none', flexDirection: 'row', minHeight: 0, width: `${ratio * 100}%` }}>
            <Box className="tm-split-divider" sx={{ width: '6px', height: '100%', cursor: 'col-resize' }} onPointerDown={handlePointerDown} onMouseEnter={handleMouseEnter} onMouseLeave={handleMouseLeave}>
                <Box className={resolveHandleClass()} sx={{ width: '4px', height: '32px' }} />
            </Box>
            <Box sx={{ flex: 1, minWidth: 0, display: 'flex' }}>
                {content}
            </Box>
        </Box>
    );
}
