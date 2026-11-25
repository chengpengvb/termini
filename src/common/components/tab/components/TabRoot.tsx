import {TabBar} from './TabBar';
import {Box} from '@mui/material';
import type {ReactNode} from 'react';
import {TabContent} from './TabContent';
import {SidePanel} from './SidePanel.tsx';
import {TabContext} from '../context/TabContext';
import {DragGhostOverlay} from './drag/DragGhostOverlay';
import type {TabManagerService} from "@/common/components/tab";
import {MobileGestureHost} from '@/page/common/gesture/MobileGestureHost.tsx';

interface TabRootProps {
    manager: TabManagerService;
}

export function TabRoot(props: TabRootProps): ReactNode {
    const {manager} = props;

    return (
        <TabContext.Provider value={{manager}}>
            <Box className="tm-app-root" sx={{position: 'fixed', top: 'var(--tm-vv-offset-top)', left: 0, right: 0, height: 'var(--tm-app-height)', display: 'flex', flexDirection: 'column'}}>
                <TabBar/>
                <Box className="tm-app-body" sx={{flex: 1, minHeight: 0, display: 'flex', flexDirection: 'row'}}>
                    <TabContent/>
                    <SidePanel/>
                </Box>
                <DragGhostOverlay/>
                <MobileGestureHost/>
            </Box>
        </TabContext.Provider>
    );
}
