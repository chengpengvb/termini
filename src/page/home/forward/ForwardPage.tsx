import {useState} from 'react';
import {Box} from '@mui/material';
import {TopBar} from './toolbar/TopBar';
import {Content} from './content/Content';
import {EditDialog} from './edit/EditDialog';
import {VIEW_MODE} from '@/common/const/Const.ts';
import {usePlatformMobile} from '@/common/context/usePlatformMobile';
import {createEmptyPortForward, PortForward} from '@/common/model/PortForward.ts';

export function ForwardPage(props: {active: boolean}) {
    const {active} = props;
    const mobileViewport = usePlatformMobile();
    const [dialogOpen, setDialogOpen] = useState<boolean>(false);
    const [viewMode, setViewMode] = useState<number>(VIEW_MODE.GRID);
    const [searchText, setSearchText] = useState<string | null>(null);
    const [portForward, setPortForward] = useState<PortForward>(createEmptyPortForward());

    const resolveViewMode = (): number => {
        if (mobileViewport) {
            return VIEW_MODE.LIST;
        }
        return viewMode;
    };

    const handleAddForward = (): void => {
        if (dialogOpen) {
            setDialogOpen(false);
            return;
        }
        setPortForward(createEmptyPortForward());
        setDialogOpen(true);
    };

    const handleBlankClick = (): void => {
        setDialogOpen(false);
    };

    const handleClose = (): void => {
        setDialogOpen(false);
    };

    const handleSave = (_value: PortForward): void => {
        setDialogOpen(false);
    };

    return (
        <Box className="tm-home-page">
            <TopBar searchText={searchText} onSearchChange={setSearchText} viewMode={viewMode} onViewModeChange={setViewMode} onAddForward={handleAddForward}/>
            <Content active={active} viewMode={resolveViewMode()} searchText={searchText} onBlankClick={handleBlankClick} switchVisible={handleAddForward} setPortForward={setPortForward} setDialogOpen={setDialogOpen}/>
            <EditDialog open={dialogOpen} portForward={portForward} onSave={handleSave} onClose={handleClose}/>
        </Box>
    );
}
