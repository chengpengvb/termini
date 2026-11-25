import {Item} from './Item';
import {Box} from '@mui/material';
import type {MouseEvent as ReactMouseEvent} from 'react';
import type {ConfigKeyPair} from '@/common/model/ConfigKeyPair.ts';

export function List(props: { keyPairs: ConfigKeyPair[]; onBlankClick: () => void, setConfigKeyPair: (value: ConfigKeyPair) => void, setDialogOpen: (open: boolean) => void }) {
    const {keyPairs, onBlankClick, setConfigKeyPair, setDialogOpen} = props;

    const handleBlankClick = (event: ReactMouseEvent<HTMLDivElement>): void => {
        if (event.currentTarget !== event.target) {
            return;
        }
        onBlankClick();
    };

    return (
        <Box className="tm-device-list" onClick={handleBlankClick}>
            {keyPairs.map(item => (
                <Item key={item.id} configKeyPair={item} setConfigKeyPair={setConfigKeyPair} setDialogOpen={setDialogOpen}/>
            ))}
        </Box>
    );
}
