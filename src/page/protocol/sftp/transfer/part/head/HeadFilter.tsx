import {Search} from "@mui/icons-material";
import {Box, InputBase} from "@mui/material";
import {useI18n} from '@/common/i18n/useI18n.ts';
import {RefObject, useEffect, useState} from "react";
import {SftpHandle} from "@/page/common/dto/sftp/SftpHandle.ts";

interface HeadFilterProps {
    filterResetToken: number;
    panelRef: RefObject<SftpHandle | null>;
}

export function HeadFilter(props: HeadFilterProps) {
    const {t} = useI18n();
    const {panelRef, filterResetToken} = props;
    const [filterText, setFilterText] = useState<string>(String());

    const applyFilterText = (value: string): void => {
        setFilterText(value);
        const query = value.trim();
        if (!query) {
            panelRef.current?.setFilterQuery(null);
            return;
        }
        panelRef.current?.setFilterQuery(query);
    };

    useEffect(() => {
        setFilterText(String());
        panelRef.current?.setFilterQuery(null);
    }, [filterResetToken, panelRef]);

    return (
        <Box className="tm-sftp-head-filter">
            <Search className="tm-sftp-head-filter-icon"/>
            <InputBase value={filterText} onChange={(event) => applyFilterText(event.target.value)} placeholder={t('common.filter')} className="tm-sftp-head-filter-input" />
        </Box>
    );
}
