import {animate, motionValue} from 'motion';
import {isNil} from '@/common/util/StringUtil.ts';
import type {StepEvent} from '@/page/common/dto/terminal/StepEvent.ts';
import type {AnimationPlaybackControlsWithThen, MotionValue} from 'motion';

export const MIN_DURATION_SEC = 0.2;
export const ACTIVE_DWELL_SEC = 0.8;

export interface StepTimelineCallbacks {
    onConnectOpen?: () => void;
    onProgressTick?: (progress: number) => void;
    onSettledChange?: (settled: boolean) => void;
    onStepEventApply?: (stepEvent: StepEvent) => void;
    onAngleTick?: (angleDeg: number, ringGlow: number) => void;
}

export interface StepTimelineOptions extends StepTimelineCallbacks {
    minDurationSec: number;
    activeDwellSec: number;
}

interface EventCheckpoint {
    kind: 'event';
    target: number;
    stepEvent: StepEvent;
}

interface OpenCheckpoint {
    kind: 'open';
}

export class StepTimeline {
    private angle: number = 0;
    private angleSign: number = 1;
    private angleRafId: number = 0;
    private lastTickMs: number = 0;
    private ringGlow: number = 0;
    private playing: boolean = false;
    private disposed: boolean = false;
    private lastNonStopApplyMs: number = 0;
    private lastProgressSample: number = 0;
    private angleLoopPaused: boolean = false;
    private dwellTimerId: number | null = null;
    private readonly opts: StepTimelineOptions;
    private lastApplied: StepEvent | null = null;
    private readonly progress: MotionValue<number> = motionValue(0);
    private currentAnim: AnimationPlaybackControlsWithThen | null = null;
    private readonly queue: Array<EventCheckpoint | OpenCheckpoint> = [];

    private readonly PROGRESS_EPSILON = 0.001;
    private readonly idleAngularSpeed: number = 257;
    private readonly minAngularSpeed: number = 225;
    private readonly maxAngularSpeed: number = 2000;
    private readonly velocityThreshold: number = 0.017;
    private readonly ringGlowDecayPerSec: number = 4.5;
    private readonly progressAngularFactor: number = 1962;

    constructor(opts: StepTimelineOptions) {
        this.opts = opts;
        this.progress.on('change', (v) => {
            if (this.disposed) {
                return;
            }
            this.opts.onProgressTick?.(v);
        });
        this.opts.onSettledChange?.(true);
        if (this.opts.onAngleTick) {
            this.startAngleLoop();
        }
    }

    pauseAngleLoop(): void {
        if (this.angleLoopPaused) {
            return;
        }
        this.angleLoopPaused = true;
        cancelAnimationFrame(this.angleRafId);
        this.angleRafId = 0;
    }

    resumeAngleLoop(): void {
        if (this.disposed || !this.angleLoopPaused || !this.opts.onAngleTick) {
            return;
        }
        this.angleLoopPaused = false;
        this.startAngleLoop();
    }

    push(stepEvent: StepEvent): void {
        if (this.disposed) {
            return;
        }
        const target = this.resolveTargetProgress(stepEvent.progress);
        this.queue.push({kind: 'event', stepEvent, target});
        this.advance();
    }

    dispose(): void {
        this.disposed = true;
        if (!isNil(this.currentAnim)) {
            this.currentAnim.stop();
            this.currentAnim = null;
        }
        if (!isNil(this.dwellTimerId)) {
            clearTimeout(this.dwellTimerId);
            this.dwellTimerId = null;
        }
        cancelAnimationFrame(this.angleRafId);
        this.queue.length = 0;
    }

    clear(): void {
        if (!isNil(this.currentAnim)) {
            this.currentAnim.stop();
            this.currentAnim = null;
        }
        if (!isNil(this.dwellTimerId)) {
            clearTimeout(this.dwellTimerId);
            this.dwellTimerId = null;
        }
        this.queue.length = 0;
        this.progress.jump(0);
        this.lastProgressSample = 0;
        this.angle = 0;
        this.angleSign = 1;
        this.ringGlow = 0;
        this.lastApplied = null;
        this.lastNonStopApplyMs = 0;
        this.playing = false;
        this.opts.onSettledChange?.(true);
    }

    private resolveTargetProgress(overallProgress: number): number {
        if (overallProgress <= 0) {
            return 0;
        }
        if (overallProgress >= 100) {
            return 1;
        }
        return overallProgress / 100;
    }

    private startAngleLoop(): void {
        if (this.disposed || this.angleLoopPaused) {
            return;
        }
        this.lastTickMs = performance.now();
        this.lastProgressSample = this.progress.get();
        const tick = (now: number): void => {
            if (this.disposed || this.angleLoopPaused) {
                return;
            }
            const dt = Math.min((now - this.lastTickMs) / 1000, 0.1);
            this.lastTickMs = now;
            const cur = this.progress.get();
            const velocity = dt > 0 ? (cur - this.lastProgressSample) / dt : 0;
            this.lastProgressSample = cur;
            const angularSpeed = this.computeAngularSpeed(velocity);
            this.angle += this.angleSign * angularSpeed * dt;
            this.ringGlow = this.advanceRingGlow(angularSpeed, dt);
            this.opts.onAngleTick?.(this.angle, this.ringGlow);
            this.angleRafId = requestAnimationFrame(tick);
        };
        this.angleRafId = requestAnimationFrame(tick);
    }

    private computeAngularSpeed(velocity: number): number {
        if (this.isFrozen()) {
            return 0;
        }
        const absV = Math.abs(velocity);
        if (absV <= this.velocityThreshold) {
            return this.idleAngularSpeed;
        }
        if (velocity > 0) {
            this.angleSign = 1;
        } else {
            this.angleSign = -1;
        }
        const raw = absV * this.progressAngularFactor;
        if (raw < this.minAngularSpeed) {
            return this.minAngularSpeed;
        }
        if (raw > this.maxAngularSpeed) {
            return this.maxAngularSpeed;
        }
        return raw;
    }

    private advanceRingGlow(angularSpeed: number, dt: number): number {
        if (angularSpeed > this.idleAngularSpeed) {
            return 1;
        }
        const next = this.ringGlow - this.ringGlowDecayPerSec * dt;
        if (next < 0) {
            return 0;
        }
        return next;
    }

    private isFrozen(): boolean {
        if (isNil(this.lastApplied)) {
            return false;
        }
        if (!this.lastApplied.stop) {
            return false;
        }
        return this.progress.get() < this.PROGRESS_EPSILON;
    }

    private advance(): void {
        if (this.disposed) {
            return;
        }
        if (this.playing) {
            return;
        }
        const next = this.queue.shift();
        if (isNil(next)) {
            this.opts.onSettledChange?.(true);
            return;
        }
        if (next.kind === 'open') {
            if (!isNil(this.opts.onConnectOpen)) {
                this.opts.onConnectOpen();
            }
            this.advance();
            return;
        }
        this.startCheckpoint(next);
    }

    private startCheckpoint(next: EventCheckpoint): void {
        this.playing = true;
        this.opts.onSettledChange?.(false);
        const dwellRemainMs = this.resolveDwellRemainMs(next.stepEvent);
        if (dwellRemainMs > 0) {
            this.dwellTimerId = window.setTimeout(() => {
                this.dwellTimerId = null;
                this.runTween(next);
            }, dwellRemainMs);
            return;
        }
        this.runTween(next);
    }

    private resolveDwellRemainMs(stepEvent: StepEvent): number {
        if (!stepEvent.stop) {
            return 0;
        }
        if (this.lastNonStopApplyMs === 0) {
            return 0;
        }
        const elapsed = performance.now() - this.lastNonStopApplyMs;
        const dwellMs = this.opts.activeDwellSec * 1000;
        const remain = dwellMs - elapsed;
        if (remain <= 0) {
            return 0;
        }
        return remain;
    }

    private runTween(next: EventCheckpoint): void {
        const distance = Math.abs(next.target - this.progress.get());
        if (distance < this.PROGRESS_EPSILON) {
            this.progress.jump(next.target);
            this.finishCheckpoint(next);
            return;
        }
        this.currentAnim = animate(this.progress, next.target, {
            type: 'tween',
            duration: this.opts.minDurationSec,
            ease: 'linear',
        });
        this.currentAnim.then(() => {
            this.currentAnim = null;
            this.finishCheckpoint(next);
        });
    }

    private finishCheckpoint(next: EventCheckpoint): void {
        if (this.disposed) {
            return;
        }
        this.opts.onStepEventApply?.(next.stepEvent);
        this.lastApplied = next.stepEvent;
        if (next.stepEvent.stop) {
            this.lastNonStopApplyMs = 0;
        } else {
            this.lastNonStopApplyMs = performance.now();
        }
        this.playing = false;
        this.advance();
    }
}
