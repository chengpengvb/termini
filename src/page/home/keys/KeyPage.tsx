import {useState} from 'react';
import {Box} from '@mui/material';
import {TopBar} from './toolbar/TopBar';
import {Content} from './content/Content';
import {EditDialog} from './edit/EditDialog';
import {VIEW_MODE} from '@/common/const/Const.ts';
import {usePlatformMobile} from '@/common/context/usePlatformMobile';
import {createEmptyConfigKeyPair, ConfigKeyPair} from '@/common/model/ConfigKeyPair.ts';

export function KeyPage(props: {active: boolean}) {
    const {active} = props;
    const mobileViewport = usePlatformMobile();
    const [dialogOpen, setDialogOpen] = useState<boolean>(false);
    const [viewMode, setViewMode] = useState<number>(VIEW_MODE.GRID);
    const [searchText, setSearchText] = useState<string | null>(null);
    const [configKeyPair, setConfigKeyPair] = useState<ConfigKeyPair>(createEmptyConfigKeyPair());

    const resolveViewMode = (): number => {
        if (mobileViewport) {
            return VIEW_MODE.LIST;
        }
        return viewMode;
    };

    const handleAddKey = (): void => {
        if (dialogOpen) {
            setDialogOpen(false);
            return;
        }
        setConfigKeyPair(createEmptyConfigKeyPair());
        setDialogOpen(true);
    };

    const handleBlankClick = (): void => {
        setDialogOpen(false);
    };

    const handleClose = (): void => {
        setDialogOpen(false);
    };

    const handleSave = (_value: ConfigKeyPair): void => {
        setDialogOpen(false);
    };

    return (
        <Box className={mobileViewport ? 'tm-home-page tm-home-key-root is-mobile' : 'tm-home-page'}>
            <TopBar searchText={searchText} onSearchChange={setSearchText} viewMode={viewMode} onViewModeChange={setViewMode} onAddKey={handleAddKey}/>
            <Content active={active} viewMode={resolveViewMode()} searchText={searchText} onBlankClick={handleBlankClick} switchVisible={handleAddKey} setConfigKeyPair={setConfigKeyPair} setDialogOpen={setDialogOpen}/>
            <EditDialog open={dialogOpen} configKeyPair={configKeyPair} onSave={handleSave} onClose={handleClose}/>
        </Box>
    );
}
