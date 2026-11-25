import {Actions} from './Actions';
import {Box} from '@mui/material';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {SearchBar} from '@/page/home/components/SearchBar.tsx';

export interface TopBarProps {
    viewMode: number;
    onAddForward: () => void;
    searchText: string | null;
    onViewModeChange: (mode: number) => void;
    onSearchChange: (value: string | null) => void;
}

export function TopBar(props: TopBarProps) {
    const {t} = useI18n();
    const {searchText, onSearchChange, viewMode, onViewModeChange, onAddForward} = props;

    return (
        <Box className="tm-home-topbar">
            <SearchBar placeholder={t('home.forward.searchPlaceholder')} searchText={searchText} onChange={onSearchChange}/>
            <Actions viewMode={viewMode} onChange={onViewModeChange} onAddForward={onAddForward}/>
        </Box>
    );
}
