import {createPortal} from 'react-dom';
import type {Terminal} from '@xterm/xterm';
import {useLayoutEffect, useRef, type RefObject} from 'react';
import {readTerminalCellMetrics} from '@/page/common/util/terminal.ts';
import {paintTerminalMagnifier, placeTerminalMagnifier, type MagnifierTarget} from '@/page/common/util/terminalMagnifier.ts';

interface Props {
    terminal: Terminal;
    hostRef: RefObject<HTMLDivElement | null>;
    target: MagnifierTarget;
}

export function TerminalSelectionMagnifier({terminal, hostRef, target}: Props) {
    const lensRef = useRef<HTMLDivElement>(null);
    const canvasRef = useRef<HTMLCanvasElement>(null);
    useLayoutEffect(() => {
        let frame = 0;
        const draw = (): void => {
            frame = 0;
            try {
                const host = hostRef.current;
                const lens = lensRef.current;
                const canvas = canvasRef.current;
                if (!host || !lens || !canvas) {
                    return;
                }
                const layout = readTerminalCellMetrics(host, terminal);
                if (!layout) {
                    lens.style.visibility = 'hidden';
                    return;
                }
                const vv = window.visualViewport;
                const left = vv?.offsetLeft ?? 0;
                const top = vv?.offsetTop ?? 0;
                const viewport = {left, top, right: left + (vv?.width ?? window.innerWidth), bottom: top + (vv?.height ?? window.innerHeight)};
                const x = layout.rowLeft + (target.col + 0.5) * layout.cellW;
                const y = layout.rowTop + (target.row - layout.viewportY + 0.5) * layout.cellH;
                const place = placeTerminalMagnifier(layout.screenRect, viewport, x, y, target.fingerX, target.fingerY);
                if (!place) {
                    lens.style.visibility = 'hidden';
                    return;
                }
                Object.assign(lens.style, {left: place.left + 'px', top: place.top + 'px', width: place.width + 'px', height: place.height + 'px', visibility: 'visible'});
                paintTerminalMagnifier(canvas, terminal, target, layout.cellW, layout.cellH, place.width, place.height);
            } catch (error) {
                console.warn(error);
            }
        };
        const schedule = (): void => {
            if (!frame) {
                frame = requestAnimationFrame(draw);
            }
        };
        draw();
        const render = terminal.onRender(schedule);
        const scroll = terminal.onScroll(schedule);
        const resize = terminal.onResize(schedule);
        const observer = new ResizeObserver(schedule);
        if (hostRef.current) {
            observer.observe(hostRef.current);
        }
        window.addEventListener('resize', schedule);
        window.visualViewport?.addEventListener('resize', schedule);
        window.visualViewport?.addEventListener('scroll', schedule);
        return () => {
            cancelAnimationFrame(frame);
            render.dispose();
            scroll.dispose();
            resize.dispose();
            observer.disconnect();
            window.removeEventListener('resize', schedule);
            window.visualViewport?.removeEventListener('resize', schedule);
            window.visualViewport?.removeEventListener('scroll', schedule);
        };
    }, [terminal, hostRef, target]);
    return createPortal(<div ref={lensRef} className="tm-terminal-selection-magnifier" aria-hidden="true">
        <canvas ref={canvasRef}/>
    </div>, document.body);
}
