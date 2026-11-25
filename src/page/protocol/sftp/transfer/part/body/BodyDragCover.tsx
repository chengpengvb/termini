import {Box} from '@mui/material';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {UploadDropIcon} from '@/assets/icon/components/UploadDropIcon.tsx';

export interface BodyDragCoverProps {
    visible: boolean;
}

export function BodyDragCover(props: BodyDragCoverProps) {
    const {t} = useI18n();
    if (!props.visible) {
        return null;
    }

    return (
        <Box className="tm-sftp-drag-cover">
            <Box className="tm-sftp-drag-cover-frame">
                <span className="tm-sftp-drag-cover-corner tm-sftp-drag-cover-corner-tl"/>
                <span className="tm-sftp-drag-cover-corner tm-sftp-drag-cover-corner-tr"/>
                <span className="tm-sftp-drag-cover-corner tm-sftp-drag-cover-corner-bl"/>
                <span className="tm-sftp-drag-cover-corner tm-sftp-drag-cover-corner-br"/>
                <Box className="tm-sftp-drag-cover-content">
                    <UploadDropIcon className="tm-sftp-drag-cover-icon"/>
                    <span className="tm-sftp-drag-cover-label">{t('protocol.sftp.dragDropHere')}</span>
                </Box>
            </Box>
        </Box>
    );
}
