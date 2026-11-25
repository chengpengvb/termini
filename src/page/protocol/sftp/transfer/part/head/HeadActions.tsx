import {Box, Button} from "@mui/material";
import {MouseEvent, RefObject} from "react";
import {useI18n} from '@/common/i18n/useI18n.ts';
import {KeyboardArrowDown} from "@mui/icons-material";
import {SftpHandle} from "@/page/common/dto/sftp/SftpHandle.ts";

interface HeadActionsProps {
    panelRef: RefObject<SftpHandle | null>;
}

export function HeadActions(props: HeadActionsProps) {
    const {t} = useI18n();
    const {panelRef} = props;

    const handleOpenMenu = (event: MouseEvent<HTMLButtonElement>): void => {
        const rect = event.currentTarget.getBoundingClientRect();
        panelRef.current?.openActionsMenu(rect.left, rect.bottom + 4);
    };

    return (
        <Box className="tm-sftp-head-actions">
            <Button disableRipple onClick={handleOpenMenu} className="tm-sftp-head-actions-trigger">
                <Box component="span" className="tm-sftp-head-actions-label">{t('common.actions')}</Box>
                <KeyboardArrowDown className="tm-sftp-head-actions-chevron"/>
            </Button>
        </Box>
    );
}
