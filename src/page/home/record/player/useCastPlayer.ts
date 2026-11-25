import {useEffect, useRef, useState} from 'react';
import type {Terminal as XTerm} from '@xterm/xterm';
import {type CastData, type CastOutput, parseCast} from '@/page/home/record/player/castParser.ts';

export const SPEED_OPTIONS = [1, 2, 4, 8];

interface EngineState {
    index: number;
    total: number;
    generation: number;
    startWall: number;
    baseElapsed: number;
    timer: number | null;
    outputs: CastOutput[];
}

export interface CastPlayer {
    total: number;
    speed: number;
    elapsed: number;
    playing: boolean;
    activity: number[];
    reset: () => void;
    restart: () => void;
    cycleSpeed: () => void;
    togglePlay: () => void;
    load: (content: string) => void;
    previewSeek: (seconds: number) => void;
    seekTo: (seconds: number, resume: boolean) => void;
    attachTerminal: (terminal: XTerm | null) => void;
}

export function useCastPlayer(): CastPlayer {
    const speedRef = useRef<number>(1);
    const [speed, setSpeed] = useState<number>(1);
    const [total, setTotal] = useState<number>(0);
    const terminalRef = useRef<XTerm | null>(null);
    const [elapsed, setElapsed] = useState<number>(0);
    const [playing, setPlaying] = useState<boolean>(false);
    const [activity, setActivity] = useState<number[]>([]);
    const stateRef = useRef<EngineState>({outputs: [], index: 0, total: 0, generation: 0, baseElapsed: 0, startWall: 0, timer: null});

    function currentElapsed(): number {
        const state = stateRef.current;
        return state.baseElapsed + (performance.now() - state.startWall) / 1000 * speedRef.current;
    }

    function resolveTotal(): number {
        return stateRef.current.total;
    }

    function invalidate(): number {
        const state = stateRef.current;
        if (state.timer !== null) {
            window.clearTimeout(state.timer);
            state.timer = null;
        }
        state.generation += 1;
        return state.generation;
    }

    function finish(): void {
        const state = stateRef.current;
        invalidate();
        state.baseElapsed = resolveTotal();
        setElapsed(resolveTotal());
        setPlaying(false);
    }

    function scheduleNext(generation: number): void {
        const state = stateRef.current;
        if (state.generation !== generation) {
            return;
        }
        if (state.index >= state.outputs.length) {
            finish();
            return;
        }
        const target = state.outputs[state.index].at;
        const delayMs = Math.max(0, (target - currentElapsed()) / speedRef.current * 1000);
        state.timer = window.setTimeout(() => {
            if (state.generation !== generation) {
                return;
            }
            state.timer = null;
            const output = state.outputs[state.index];
            state.index += 1;
            terminalRef.current?.write(output.data);
            scheduleNext(generation);
        }, delayMs);
    }

    function play(): void {
        const state = stateRef.current;
        const generation = invalidate();
        state.startWall = performance.now();
        setPlaying(true);
        scheduleNext(generation);
    }

    function pause(): void {
        const state = stateRef.current;
        state.baseElapsed = currentElapsed();
        invalidate();
        setPlaying(false);
    }

    function restart(): void {
        const state = stateRef.current;
        invalidate();
        terminalRef.current?.reset();
        state.index = 0;
        state.baseElapsed = 0;
        setElapsed(0);
        play();
    }

    function togglePlay(): void {
        if (playing) {
            pause();
            return;
        }
        if (stateRef.current.index >= stateRef.current.outputs.length) {
            restart();
            return;
        }
        play();
    }

    function cycleSpeed(): void {
        const state = stateRef.current;
        const wasPlaying = playing;
        if (wasPlaying) {
            state.baseElapsed = currentElapsed();
            invalidate();
        }
        const next = SPEED_OPTIONS[(SPEED_OPTIONS.indexOf(speedRef.current) + 1) % SPEED_OPTIONS.length];
        speedRef.current = next;
        setSpeed(next);
        if (wasPlaying) {
            play();
        }
    }

    function previewSeek(seconds: number): void {
        invalidate();
        setElapsed(seconds);
    }

    function seekTo(seconds: number, resume: boolean): void {
        const state = stateRef.current;
        const terminal = terminalRef.current;
        invalidate();
        if (terminal === null) {
            return;
        }
        terminal.reset();
        let index = 0;
        let chunk = '';
        while (index < state.outputs.length && state.outputs[index].at <= seconds) {
            chunk += state.outputs[index].data;
            index += 1;
        }
        if (chunk.length > 0) {
            terminal.write(chunk);
        }
        state.index = index;
        state.baseElapsed = seconds;
        setElapsed(seconds);
        if (resume && index < state.outputs.length) {
            play();
            return;
        }
        setPlaying(false);
    }

    function load(content: string): void {
        const data: CastData = parseCast(content);
        const state = stateRef.current;
        invalidate();
        state.outputs = data.outputs;
        state.total = data.total;
        state.index = 0;
        state.baseElapsed = 0;
        setTotal(data.total);
        setActivity(data.activity);
        setElapsed(0);
        play();
    }

    function reset(): void {
        const state = stateRef.current;
        invalidate();
        state.outputs = [];
        state.total = 0;
        state.index = 0;
        state.baseElapsed = 0;
        setTotal(0);
        setActivity([]);
        setElapsed(0);
        setPlaying(false);
    }

    function attachTerminal(terminal: XTerm | null): void {
        terminalRef.current = terminal;
    }

    useEffect(() => {
        if (!playing) {
            return;
        }
        let frame = 0;
        const tick = (): void => {
            setElapsed(Math.min(resolveTotal(), currentElapsed()));
            frame = window.requestAnimationFrame(tick);
        };
        frame = window.requestAnimationFrame(tick);
        return () => window.cancelAnimationFrame(frame);
    }, [playing]);

    return {total, speed, elapsed, playing, activity, load, reset, restart, cycleSpeed, togglePlay, previewSeek, seekTo, attachTerminal};
}
