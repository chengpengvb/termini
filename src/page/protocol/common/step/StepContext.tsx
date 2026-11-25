import {Box} from "@mui/material";
import type {ReactNode, RefObject} from "react";
import {useEffect, useRef, useState} from "react";
import {ConnectService} from "@/service/ConnectService.ts";
import type {ReqDevice} from "@/common/model/req/ReqDevice.ts";
import {StepEvent} from "@/page/common/dto/terminal/StepEvent.ts";
import {BottomBox} from "@/page/protocol/common/step/bottom/BottomBox.tsx";
import type {StepHeadHandle} from "@/page/protocol/common/step/StepHead.tsx";
import {StepLog, StepLogHandle} from "@/page/protocol/common/step/log/StepLog.tsx";
import {ACTIVE_DWELL_SEC, MIN_DURATION_SEC, StepTimeline} from "@/page/common/util/StepTimeline.ts";

export interface StepContextHandle {
    clear: () => void;
}

export interface StepContextProps {
    sessionId: string;
    authPage?: ReactNode;
    reqDevice: ReqDevice | null;
    connectService?: ConnectService;
    setOnOpened: (value: boolean) => void;//用来控制显式的,隐藏弹出框,这个比打开事件要晚
    stepHeadRef: RefObject<StepHeadHandle | null>;
}

export function StepContext(props: StepContextProps) {
    const [failed, setFailed] = useState<boolean>(false);
    const stepLogRef = useRef<StepLogHandle | null>(null);
    const timelineRef = useRef<StepTimeline | null>(null);
    const [showBottomBar, setShowBottomBar] = useState<boolean>(false);
    const {connectService, stepHeadRef, reqDevice, setOnOpened, sessionId, authPage} = props;

    const clear = (): void => {
        setFailed(false);
        setShowBottomBar(false);
        timelineRef.current?.clear();
        stepLogRef.current?.clear();
    };

    const handleStepEventApply = (next: StepEvent): void => {
        const log = stepLogRef.current;
        if (!log) {
            return;
        }
        log.pushStepEvent(next);
        if (!next.error) {
            return;
        }
        setFailed(true);
        if (next.progress <= 0) {
            timelineRef.current?.clear()
        }
    };

    const stepContextHandle: StepContextHandle = {clear};

    useEffect(() => {
        const timeline = timelineRef.current ?? new StepTimeline({
            minDurationSec: MIN_DURATION_SEC,
            activeDwellSec: ACTIVE_DWELL_SEC,
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
                return
            }
            setShowBottomBar(true);
        });
        return () => {
            off?.();
        };
    }, [connectService, sessionId]);

    if (!reqDevice) {
        return null
    }
    return (
        <Box sx={{width: '100%', height: '100%'}}>
            <StepLog ref={stepLogRef} failed={failed} setOnOpened={setOnOpened}/>
            <BottomBox failed={showBottomBar} sessionId={sessionId} reqDevice={reqDevice} connectService={connectService} stepHeadRef={stepHeadRef} stepContextHandle={stepContextHandle}/>
            {authPage}
        </Box>
    );
}
