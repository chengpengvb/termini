import {Item} from './Item';
import {Box} from '@mui/material';
import type {MouseEvent as ReactMouseEvent} from 'react';
import type {ReqDevice} from '@/common/model/req/ReqDevice.ts';

export function Grid(props: { devices: ReqDevice[]; onBlankClick: () => void, setReqDevice: (reqDevice: ReqDevice) => void, setDrawerVisible: (visible: boolean) => void }) {
    const {devices, onBlankClick, setReqDevice, setDrawerVisible} = props;

    const handleBlankClick = (event: ReactMouseEvent<HTMLDivElement>): void => {
        if (event.currentTarget !== event.target) {
            return;
        }
        onBlankClick();
    };

    return (
        <Box className="tm-device-grid" onClick={handleBlankClick}>
            {devices.map(reqDevice => (
                <Item key={reqDevice.device.id} setReqDevice={setReqDevice} reqDevice={reqDevice} setDrawerVisible={setDrawerVisible}/>
            ))}
        </Box>
    );
}
