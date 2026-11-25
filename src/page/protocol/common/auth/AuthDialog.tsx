import { Dialog } from '@mui/material';
import { ConnectService } from '@/service/ConnectService.ts';
import type { ReqDevice } from '@/common/model/req/ReqDevice.ts';
import { AlertDialog } from '@/common/components/dialog/AlertDialog.tsx';
import { RespSocketContext } from '@/common/model/resp/RespWsContext.ts';
import { Dispatch, ReactNode, SetStateAction, useEffect, useState } from 'react';
import { resolveSheetDialogProps } from '@/page/common/util/scopedDialogUtil.ts';

export interface AuthStepContext {
  reqDevice: ReqDevice;
  handleClose: () => void;
  respSocketContext: RespSocketContext;
  setAlertInfo: Dispatch<SetStateAction<{ showAlert: boolean; message: string | null }>>;
}

export interface AuthDialogProps {
  sessionId: string;
  reqDevice: ReqDevice;
  connectService?: ConnectService;
  renderStep: (context: AuthStepContext) => ReactNode;
}

export function AuthDialog(props: AuthDialogProps) {
  const { sessionId, reqDevice, connectService, renderStep } = props;

  const [open, setOpen] = useState(false);
  const [respSocketContext, setRespSocketContext] = useState<RespSocketContext | null>(null);
  const [alertInfo, setAlertInfo] = useState<{ showAlert: boolean; message: string | null }>({ showAlert: false, message: null });

  const handleAuthStep = (context: RespSocketContext) => {
    setRespSocketContext(context);
    setOpen(true);
  };

  const handleClose = () => {
    setOpen(false);
    setRespSocketContext(null);
  };

  const handleAlertClose = (): void => {
    setAlertInfo({ showAlert: false, message: null });
  };

  const resolveContent = (): ReactNode => {
    if (!respSocketContext) {
      return null;
    }
    return renderStep({ reqDevice, respSocketContext, handleClose, setAlertInfo });
  };

  useEffect(() => {
    const unsubscribe = connectService?.addAuthListener(sessionId, (context: RespSocketContext) => {
      handleAuthStep(context);
    });
    return () => {
      unsubscribe?.();
    };
  }, [sessionId, connectService]);

  return (
    <>
      <Dialog open={open} disableEscapeKeyDown {...resolveSheetDialogProps({ className: 'tm-ssh-auth-dialog-paper' })}>
        {resolveContent()}
      </Dialog>
      <AlertDialog duration={1500} open={alertInfo.showAlert} handleClose={handleAlertClose} message={alertInfo.message} type="error" />
    </>
  );
}
