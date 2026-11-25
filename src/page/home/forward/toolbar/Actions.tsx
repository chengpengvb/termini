import {Box} from '@mui/material';
import {SwapHoriz} from '@mui/icons-material';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {ViewMode} from '../../components/ViewMode';
import {ActionPill} from '../../components/ActionPill';

export interface ActionsProps {
    viewMode: number;
    onAddForward: () => void;
    onChange: (mode: number) => void;
}

export function Actions(props: ActionsProps) {
    const {t} = useI18n();
    const {viewMode, onChange, onAddForward} = props;

    return (
        <Box className="tm-home-actions-row">
            <ActionPill icon={<SwapHoriz/>} label={t('home.forward.addForward')} primary onClick={onAddForward}/>
            <Box className="tm-home-actions-spacer"/>
            <ViewMode viewMode={viewMode} onChange={onChange}/>
        </Box>
    );
}
