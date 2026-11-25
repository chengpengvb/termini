import React, {RefObject} from "react";
import Alert from '@mui/material/Alert';
import Portal from '@mui/material/Portal';
import Close from '@mui/icons-material/Close';
import IconButton from '@mui/material/IconButton';
import {usePlatformMobile} from '@/common/context/usePlatformMobile';
import Snackbar, {SnackbarCloseReason} from '@mui/material/Snackbar';

export type AlertColor = 'success' | 'info' | 'warning' | 'error';

interface AlertDialogProps {
    open: boolean;
    duration: number;
    type: AlertColor
    message: string | null;
    containerRef?: RefObject<HTMLElement | null>;
    handleClose: (event?: React.SyntheticEvent | Event, reason?: SnackbarCloseReason) => void
}

export function AlertDialog(props: AlertDialogProps) {
    const {open, handleClose, message, type, duration, containerRef} = props;
    const mobileViewport = usePlatformMobile();

    function resolveSnackbarClassName(): string {
        if (containerRef) {
            return 'tm-alert-snackbar tm-alert-snackbar-contained';
        }
        return 'tm-alert-snackbar';
    }

    function handleCloseClick(event: React.MouseEvent<HTMLButtonElement>): void {
        handleClose(event);
    }

    return (
        <Portal container={containerRef == null ? document.body : containerRef.current}>
            <Snackbar
                className={resolveSnackbarClassName()}
                anchorOrigin={mobileViewport ? {vertical: 'top', horizontal: 'center'} : {vertical: 'bottom', horizontal: 'right'}}
                open={open}
                autoHideDuration={duration}
                onClose={handleClose}
                transitionDuration={mobileViewport ? 0 : undefined}
            >
                <Alert
                    className="tm-alert-banner"
                    variant={mobileViewport ? 'standard' : 'outlined'}
                    severity={type}
                    action={
                        <IconButton className="tm-alert-close" disableRipple color="inherit" aria-label="close" onClick={handleCloseClick}>
                            <Close/>
                        </IconButton>
                    }
                >
                    {message}
                </Alert>
            </Snackbar>
        </Portal>
    );
}
