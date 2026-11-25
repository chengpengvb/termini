import { Box } from '@mui/material';
import type { ReactNode } from 'react';
import { useEffect, useRef, useState } from 'react';
import { ConnectService } from '@/service/ConnectService.ts';
import type { ReqDevice } from '@/common/model/req/ReqDevice.ts';
import { markPaneTitleLost } from '@/page/common/util/terminal.ts';
import { useTabManagerService } from '@/common/context/useAppHooks.tsx';
import { ConnectLost } from '@/page/protocol/common/step/ConnectLost.tsx';
import { DeviceHead } from '@/page/protocol/common/select/DeviceHead.tsx';
import { StepContext } from '@/page/protocol/common/step/StepContext.tsx';
import { StepHead, type StepHeadHandle } from '@/page/protocol/common/step/StepHead.tsx';

export interface StepPageProps {
  protocol: number;
  sessionId: string;
  authPage?: ReactNode;
  reqDevice: ReqDevice;
  connectService: ConnectService;
}

const RECONNECT_DELAYS_SEC = [1, 2, 4, 8, 16, 30];

export function StepPage(props: StepPageProps) {
  const attemptRef = useRef<number>(0);
  const autoRef = useRef<boolean>(false);
  const awaitingRef = useRef<boolean>(false);
  const tickRef = useRef<number | null>(null);
  const stepHeadRef = useRef<StepHeadHandle>(null);
  const [lost, setLost] = useState<boolean>(false);
  const tabManagerService = useTabManagerService();
  const [attempt, setAttempt] = useState<number>(0);
  const [retryTick, setRetryTick] = useState<number>(0);
  const [countdown, setCountdown] = useState<number>(0);
  const [hideDialog, setHideDialog] = useState<boolean>(false);
  const { connectService, reqDevice, sessionId, authPage, protocol } = props;

  const cancelCountdown = (): void => {
    if (tickRef.current === null) {
      return;
    }
    window.clearInterval(tickRef.current);
    tickRef.current = null;
    setCountdown(0);
  };

  const doReconnect = (): void => {
    cancelCountdown();
    setLost(false);
    awaitingRef.current = true;
    stepHeadRef.current?.clear();
    setRetryTick(prev => prev + 1);
    setHideDialog(false);
    connectService.noticeRetry(sessionId);
  };

  const scheduleReconnect = (): void => {
    cancelCountdown();
    const index = attemptRef.current;
    if (index >= RECONNECT_DELAYS_SEC.length) {
      autoRef.current = false;
      setLost(false);
      return;
    }
    attemptRef.current = index + 1;
    setAttempt(index + 1);
    setLost(true);
    let remain = RECONNECT_DELAYS_SEC[index];
    setCountdown(remain);
    tickRef.current = window.setInterval(() => {
      remain -= 1;
      if (remain > 0) {
        setCountdown(remain);
        return;
      }
      doReconnect();
    }, 1000);
  };

  const handleManualRetry = (): void => {
    autoRef.current = false;
    attemptRef.current = 0;
    setAttempt(0);
    doReconnect();
  };

  const handleStopAuto = (): void => {
    autoRef.current = false;
    cancelCountdown();
  };

  useEffect(() => {
    return connectService.addCloseListener(sessionId, () => {
      setLost(true);
      autoRef.current = true;
      attemptRef.current = 0;
      scheduleReconnect();
    });
  }, [sessionId, connectService]);

  useEffect(() => {
    return connectService.addConnectStepListener(sessionId, next => {
      if (!next.error || !autoRef.current || !awaitingRef.current) {
        return;
      }
      awaitingRef.current = false;
      scheduleReconnect();
    });
  }, [sessionId, connectService]);

  useEffect(() => {
    if (!hideDialog) {
      return;
    }
    autoRef.current = false;
    awaitingRef.current = false;
    attemptRef.current = 0;
    setAttempt(0);
    cancelCountdown();
  }, [hideDialog]);

  useEffect(() => {
    markPaneTitleLost(tabManagerService, sessionId, lost);
  }, [lost, sessionId, tabManagerService]);

  useEffect(() => {
    return () => {
      cancelCountdown();
    };
  }, []);

  if (lost) {
    return <ConnectLost attempt={attempt} maxAttempt={RECONNECT_DELAYS_SEC.length} countdown={countdown} onRetry={handleManualRetry} onStop={handleStopAuto} />;
  }

  if (hideDialog) {
    return null;
  }

  return (
    <Box sx={{ position: 'absolute', inset: 0, zIndex: 20 }}>
      <Box sx={{ position: 'absolute', inset: 0 }}>
        <Box className="tm-protocol-frame-root">
          <Box className="tm-protocol-frame tm-protocol-frame--select">
            <DeviceHead reqDevice={reqDevice} protocol={protocol} />
            <StepHead ref={stepHeadRef} connectService={connectService} sessionId={sessionId} />
            <Box className="tm-protocol-frame-body">
              <StepContext key={retryTick} sessionId={sessionId} authPage={authPage} reqDevice={reqDevice} connectService={connectService} setOnOpened={setHideDialog} stepHeadRef={stepHeadRef} />
            </Box>
          </Box>
        </Box>
      </Box>
    </Box>
  );
}
