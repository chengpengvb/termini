import {useState} from 'react';
import {AddDialog} from './dialog/AddDialog.tsx';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {Box, Button, ButtonBase, InputBase} from '@mui/material';
import {GridViewOutlined, Search, ViewAgendaOutlined} from '@mui/icons-material';
import {JOB_MONITOR_VIEW, type JobMonitorView} from '@/page/common/model/JobModel';

export interface ToolbarProps {
    view: JobMonitorView;
    onAdded: () => void;
    searchText: string;
    onSearchChange: (value: string) => void;
    onViewChange: (view: JobMonitorView) => void;
}

export function Toolbar(props: ToolbarProps) {
    const {view, onAdded, searchText, onSearchChange, onViewChange} = props;
    const [addOpen, setAddOpen] = useState(false);
    const {t} = useI18n();

    return (
        <>
            <Box className="tm-home-searchbar">
                <Search className="tm-home-search-icon"/>
                <InputBase className="tm-home-search-input-root" inputProps={{className: 'tm-home-search-input'}} placeholder={t('home.job.searchPlaceholder')} value={searchText} onChange={event => onSearchChange(event.target.value)}/>
                <Box className="tm-job-view-toggle">
                    <ButtonBase disableRipple className={view === JOB_MONITOR_VIEW.LIST ? 'tm-job-view-btn is-active' : 'tm-job-view-btn'} title={t('home.job.viewList')} aria-label={t('home.job.viewList')} onClick={() => onViewChange(JOB_MONITOR_VIEW.LIST)}>
                        <ViewAgendaOutlined sx={{fontSize: 15}}/>
                    </ButtonBase>
                    <ButtonBase disableRipple className={view === JOB_MONITOR_VIEW.GRID ? 'tm-job-view-btn is-active' : 'tm-job-view-btn'} title={t('home.job.viewGrid')} aria-label={t('home.job.viewGrid')} onClick={() => onViewChange(JOB_MONITOR_VIEW.GRID)}>
                        <GridViewOutlined sx={{fontSize: 15}}/>
                    </ButtonBase>
                </Box>
                <Button variant="text" disableRipple className="tm-btn tm-btn-primary tm-btn-compact tm-job-add-btn" onClick={() => setAddOpen(true)}>
                    {t('home.job.addDevice')}
                </Button>
            </Box>
            <AddDialog open={addOpen} onClose={() => setAddOpen(false)} onAdded={onAdded}/>
        </>
    );
}
