import {Box, Typography} from '@mui/material';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {ReactNode, Ref, useImperativeHandle} from 'react';
import {ConnectService} from "@/service/ConnectService.ts";
import {Fragment, useEffect, useRef, useState} from 'react';
import CheckRoundedIcon from '@mui/icons-material/CheckRounded';
import {StepEvent} from "@/page/common/dto/terminal/StepEvent.ts";
import {ACTIVE_DWELL_SEC, MIN_DURATION_SEC, StepTimeline} from '@/page/common/util/StepTimeline.ts';

export interface StepHeadHandle {
    clear: () => void;
}

export interface StepHeadProps {
    paused?: boolean;
    sessionId: string;
    ref?: Ref<StepHeadHandle>;
    connectService?: ConnectService;
}

export function StepHead(props: StepHeadProps): ReactNode {
    const STEP_SIZE = 3;

    const STEP_STATUS = {
        PENDING: 0,
        ACTIVE: 1,
        SUCCESS: 2,
        ERROR: 3,
    } as const;

    const STATUS_CLASS: Record<number, string> = {
        [STEP_STATUS.PENDING]: 'is-pending',
        [STEP_STATUS.ACTIVE]: 'is-active',
        [STEP_STATUS.SUCCESS]: 'is-success',
        [STEP_STATUS.ERROR]: 'is-error',
    };

    const {ref, connectService, paused, sessionId} = props;
    const {t} = useI18n();
    const timelineRef = useRef<StepTimeline | null>(null);
    const containerRef = useRef<HTMLDivElement | null>(null);
    const stepHeadHandleRef = useRef<StepHeadHandle | null>(null);
    const [displayProgress, setDisplayProgress] = useState<number>(0);
    const [stepEvent, setStepEvent] = useState<StepEvent>({stop: false, error: false, progress: 0, opened: false});

    const STEPS: ReadonlyArray<{ index: number; label: string }> = [
        {index: 1, label: t('protocol.step.probe')},
        {index: 2, label: t('protocol.step.auth')},
        {index: 3, label: t('protocol.step.session')},
    ];

    const resolveCurrentStep = (overallProgress: number): number => {
        if (overallProgress <= 0) {
            return 0;
        }
        if (overallProgress >= 100) {
            return STEP_SIZE;
        }
        const segmentSize = 100 / STEP_SIZE;
        const computed = Math.floor(overallProgress / segmentSize) + 1;
        if (computed > STEP_SIZE) {
            return STEP_SIZE;
        }
        return computed;
    };

    const resolveLineScale = (lineIndex: number): number => {
        const segmentProgress = displayProgress * STEP_SIZE;
        const progressOffset = segmentProgress - lineIndex;
        if (progressOffset <= 0) {
            return 0;
        }
        if (progressOffset >= 1) {
            return 1;
        }
        return progressOffset;
    };

    const resolveStepStatus = (stepIndex: number): number => {
        const currentStep = resolveCurrentStep(stepEvent.progress);
        if (currentStep === 0) {
            if (stepIndex === 1 && !stepEvent.stop) {
                return STEP_STATUS.ACTIVE;
            }
            return STEP_STATUS.PENDING;
        }
        if (stepIndex < currentStep) {
            return STEP_STATUS.SUCCESS;
        }
        if (stepIndex > currentStep) {
            return STEP_STATUS.PENDING;
        }
        if (stepEvent.error) {
            return STEP_STATUS.ERROR;
        }
        if (stepIndex === STEP_SIZE && stepEvent.progress >= 100) {
            return STEP_STATUS.SUCCESS;
        }
        return STEP_STATUS.ACTIVE;
    };

    const resolveLineStatus = (leftStepIndex: number): number => {
        const currentStep = resolveCurrentStep(stepEvent.progress);
        if (leftStepIndex < currentStep) {
            return STEP_STATUS.SUCCESS;
        }
        if (leftStepIndex > currentStep) {
            return STEP_STATUS.PENDING;
        }
        if (stepEvent.error) {
            return STEP_STATUS.ERROR;
        }
        return STEP_STATUS.ACTIVE;
    };

    const resolveRingClass = (status: number): string | null => {
        const isIdle = stepEvent.progress <= 0 && !stepEvent.stop;
        if (status === STEP_STATUS.ACTIVE && !stepEvent.error) {
            if (isIdle) {
                return 'is-ring-waiting';
            }
            return 'is-ring-active';
        }
        if (status === STEP_STATUS.ERROR) {
            return 'is-ring-error';
        }
        return null;
    };

    const buildCircleClass = (status: number): string => {
        const parts: string[] = ['tm-protocol-progress-circle', STATUS_CLASS[status]];
        const ringClass = resolveRingClass(status);
        if (ringClass) {
            parts.push(ringClass);
        }
        return parts.join(' ');
    };

    const buildLineFillClass = (leftStepIndex: number): string => {
        return `tm-protocol-progress-line-fill ${STATUS_CLASS[resolveLineStatus(leftStepIndex)]}`;
    };

    const applyAngleStyle = (angleDeg: number, ringGlow: number): void => {
        const el = containerRef.current;
        if (!el) {
            return;
        }
        el.style.setProperty('--tm-spin-angle', `${angleDeg}deg`);
        el.style.setProperty('--tm-ring-glow', `${ringGlow}`);
    };

    const handleStepEventApply = (stepEvent: StepEvent): void => {
        setStepEvent(stepEvent);
        if (!stepEvent.error) {
            return
        }
        if (stepEvent.progress <= 0) {
            timelineRef.current?.clear()
        }
    };

    const clear = (): void => {
        timelineRef.current?.clear();
    }

    useImperativeHandle(ref, () => {
        const handle: StepHeadHandle = {clear};
        stepHeadHandleRef.current = handle;
        return handle;
    });

    useEffect(() => {
        const timeline = timelineRef.current ?? new StepTimeline({
            onAngleTick: applyAngleStyle,
            minDurationSec: MIN_DURATION_SEC,
            activeDwellSec: ACTIVE_DWELL_SEC,
            onProgressTick: setDisplayProgress,
            onStepEventApply: handleStepEventApply,
        });
        timelineRef.current = timeline;
        timeline.resumeAngleLoop();
        return () => {
            timeline.pauseAngleLoop();
        };
    }, []);

    useEffect(() => {
        const off = connectService?.addConnectStepListener(sessionId, (next: StepEvent) => {
            timelineRef.current?.push(next);
            if (!next.error) {
                return;
            }
            timelineRef.current?.push({stop: next.stop, error: next.error, progress: 0, opened: false});
        });
        return () => {
            off?.();
        };
    }, [connectService, sessionId]);

    useEffect(() => {
        const timeline = timelineRef.current;
        if (!timeline) {
            return;
        }
        if (paused) {
            timeline.pauseAngleLoop();
            return;
        }
        timeline.resumeAngleLoop();
    }, [paused]);

    return (
        <Box ref={containerRef} className="tm-protocol-progress">
            {STEPS.map((step, lineIndex) => {
                const status = resolveStepStatus(step.index);
                const stateClass = STATUS_CLASS[status];
                const showCheck = status === STEP_STATUS.SUCCESS;
                const isLast = lineIndex === STEPS.length - 1;
                return (
                    <Fragment key={step.index}>
                        <Box className="tm-protocol-progress-step">
                            <Box className={buildCircleClass(status)}>
                                {showCheck && <CheckRoundedIcon/>}
                                {!showCheck && <Typography component="span" className="tm-protocol-progress-num">{step.index}</Typography>}
                            </Box>
                            <Typography className={`tm-protocol-progress-label ${stateClass}`}>{step.label}</Typography>
                        </Box>
                        {!isLast && (
                            <Box className="tm-protocol-progress-line">
                                <Box className={buildLineFillClass(step.index)} style={{transform: `scaleX(${resolveLineScale(lineIndex)})`}}/>
                            </Box>
                        )}
                    </Fragment>
                );
            })}
        </Box>
    );
}
