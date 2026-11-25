import {Item} from './Item';
import {Box} from '@mui/material';
import type {MouseEvent as ReactMouseEvent} from 'react';
import type {PortForward} from '@/common/model/PortForward.ts';

export function Grid(props: { forwards: PortForward[]; onBlankClick: () => void, setPortForward: (value: PortForward) => void, setDialogOpen: (open: boolean) => void }) {
    const {forwards, onBlankClick, setPortForward, setDialogOpen} = props;

    const handleBlankClick = (event: ReactMouseEvent<HTMLDivElement>): void => {
        if (event.currentTarget !== event.target) {
            return;
        }
        onBlankClick();
    };

    return (
        <Box className="tm-device-grid" onClick={handleBlankClick}>
            {forwards.map(item => (
                <Item key={item.id} portForward={item} setPortForward={setPortForward} setDialogOpen={setDialogOpen}/>
            ))}
        </Box>
    );
}
