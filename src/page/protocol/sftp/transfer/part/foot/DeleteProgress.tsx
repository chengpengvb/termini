import {useEffect, useState} from 'react';
import Close from '@mui/icons-material/Close';
import {isNotEmpty} from '@/common/util/StringUtil.ts';
import {Box, IconButton, LinearProgress} from '@mui/material';
import DeleteOutline from '@mui/icons-material/DeleteOutline';
import {RespSftpDeleteProgress} from '@/common/model/resp/RespSftp.ts';

export interface DeleteProgressProps {
    onClose: () => void;
    progress: RespSftpDeleteProgress;
}

export function DeleteProgress(props: DeleteProgressProps) {
    const {progress, onClose} = props;
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

    useEffect(() => {
        if (isNotEmpty(incomingFile)) {
            setFile(incomingFile);
        }
    }, [incomingFile]);

    return (
        <Box className={finish ? 'tm-sftp-transfer-progress tm-sftp-transfer-progress-finish' : 'tm-sftp-transfer-progress'}>
            {file ? (
                <>
                    <DeleteOutline className="tm-sftp-transfer-progress-icon" fontSize="small"/>
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
            </Box>
            <IconButton className="tm-sftp-foot-close" size="small" onClick={onClose}>
                <Close fontSize="small"/>
            </IconButton>
        </Box>
    );
}
