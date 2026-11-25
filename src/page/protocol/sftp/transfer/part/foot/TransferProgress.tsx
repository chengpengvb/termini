import {useEffect, useState} from 'react';
import Close from '@mui/icons-material/Close';
import {isNotEmpty} from '@/common/util/StringUtil.ts';
import {Box, IconButton, LinearProgress} from '@mui/material';
import {RespSftpTfProgress} from '@/common/model/resp/RespSftp.ts';
import {formatTransferSpeed, getFileIconByExtension} from '@/page/common/util/sftpUtil.ts';

export interface TransferProgressProps {
    onClose: () => void;
    progress: RespSftpTfProgress;
}

export function TransferProgress(props: TransferProgressProps) {
    const {progress, onClose} = props;
    const speed = progress.speed;
    const finish = progress.finish;
    const incomingFile = progress.file;
    const [file, setFile] = useState<string | null>(null);
    const percent = (() => {
        const raw = progress.percent;
        if (!Number.isFinite(raw)) {
            return 0;
        }
        return Math.min(100, Math.max(0, raw));
    })();

    const resolveFileIcon = () => {
        if (!file) {
            return null;
        }
        const IconComponent = getFileIconByExtension(file);
        return <IconComponent className="tm-sftp-transfer-progress-icon" fontSize="small"/>;
    };

    const handleClose = () => {
        onClose();
    }

    useEffect(() => {
        if (isNotEmpty(incomingFile)) {
            setFile(incomingFile);
        }
    }, [incomingFile]);

    return (
        <Box className={finish ? 'tm-sftp-transfer-progress tm-sftp-transfer-progress-finish' : 'tm-sftp-transfer-progress'}>
            {file ? (
                <>
                    {resolveFileIcon()}
                    <Box className="tm-sftp-transfer-progress-file" title={file}>
                        {file}
                    </Box>
                </>
            ) : null}
            <Box className="tm-sftp-transfer-progress-right">
                <Box className="tm-sftp-transfer-progress-bar">
                    <LinearProgress className="tm-sftp-transfer-progress-linear" variant="determinate" value={percent}/>
                </Box>
                <Box className="tm-sftp-transfer-progress-percent">{Math.round(percent)}%</Box>
                {!finish && speed > 0 ? <Box className="tm-sftp-transfer-progress-speed">{formatTransferSpeed(speed)}</Box> : null}
            </Box>
            <IconButton className="tm-sftp-foot-close" size="small" onClick={handleClose}>
                <Close fontSize="small"/>
            </IconButton>
        </Box>
    );
}
