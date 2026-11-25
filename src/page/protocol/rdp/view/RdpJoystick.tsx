import {Box} from '@mui/material';
import {useRef, useState, type PointerEvent as ReactPointerEvent} from 'react';

const RADIUS = 50;
const THUMB = 20;
const STEP = 3.9;
const TICK_MS = 20;

export interface RdpJoystickProps {
    onMove: (dx: number, dy: number) => void;
}

export function RdpJoystick(props: RdpJoystickProps) {
    const {onMove} = props;
    const lastStampRef = useRef(0);
    const onMoveRef = useRef(onMove);
    const padRef = useRef<HTMLDivElement>(null);
    const offsetRef = useRef<{ x: number; y: number }>({x: 0, y: 0});
    const [offset, setOffset] = useState<{ x: number; y: number }>({x: 0, y: 0});
    onMoveRef.current = onMove;

    const emitOffset = (stamp: number): void => {
        const current = offsetRef.current;
        const elapsed = lastStampRef.current === 0 ? TICK_MS : stamp - lastStampRef.current;
        lastStampRef.current = stamp;
        if (current.x === 0 && current.y === 0) {
            return;
        }
        const scale = elapsed / TICK_MS;
        onMoveRef.current((current.x / RADIUS) * STEP * scale, (current.y / RADIUS) * STEP * scale);
    };

    const applyLocal = (event: ReactPointerEvent<HTMLDivElement>): void => {
        const box = padRef.current;
        if (!box) {
            return;
        }
        const rect = box.getBoundingClientRect();
        const rawX = event.clientX - (rect.left + RADIUS);
        const rawY = event.clientY - (rect.top + RADIUS);
        const dist = Math.hypot(rawX, rawY);
        const next = dist <= RADIUS ? {x: rawX, y: rawY} : {x: (rawX / dist) * RADIUS, y: (rawY / dist) * RADIUS};
        offsetRef.current = next;
        setOffset(next);
    };

    const handleDown = (event: ReactPointerEvent<HTMLDivElement>): void => {
        event.preventDefault();
        event.stopPropagation();
        event.currentTarget.setPointerCapture(event.pointerId);
        lastStampRef.current = 0;
        applyLocal(event);
        emitOffset(event.timeStamp);
    };

    const handleMove = (event: ReactPointerEvent<HTMLDivElement>): void => {
        if (!event.currentTarget.hasPointerCapture(event.pointerId)) {
            return;
        }
        event.preventDefault();
        applyLocal(event);
        emitOffset(event.timeStamp);
    };

    const handleUp = (event: ReactPointerEvent<HTMLDivElement>): void => {
        if (event.currentTarget.hasPointerCapture(event.pointerId)) {
            event.currentTarget.releasePointerCapture(event.pointerId);
        }
        lastStampRef.current = 0;
        offsetRef.current = {x: 0, y: 0};
        setOffset({x: 0, y: 0});
    };

    return (
        <Box ref={padRef} className={`tm-rdp-joystick${offset.x !== 0 || offset.y !== 0 ? ' is-held' : ''}`} onPointerDown={handleDown} onPointerMove={handleMove} onPointerUp={handleUp} onPointerCancel={handleUp}>
            <Box className="tm-rdp-joystick-thumb" style={{width: THUMB * 2, height: THUMB * 2, margin: -THUMB, transform: `translate(${offset.x}px, ${offset.y}px)`}}/>
        </Box>
    );
}
