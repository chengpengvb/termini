import {Box, ButtonBase} from '@mui/material';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {VIEW_MODE} from "@/common/const/Const.ts";
import {GridView, ViewList} from '@mui/icons-material';

export interface ViewModeProps {
    viewMode: number;
    onChange: (mode: number) => void;
}

export function ViewMode(props: ViewModeProps) {
    const {viewMode, onChange} = props;
    const {t} = useI18n();
    const gridActive = viewMode === VIEW_MODE.GRID;
    const listActive = viewMode === VIEW_MODE.LIST;

    const resolveGridClassName = (): string => {
        if (gridActive) {
            return 'tm-home-viewmode-btn tm-home-viewmode-btn--left is-active';
        }
        return 'tm-home-viewmode-btn tm-home-viewmode-btn--left';
    };

    const resolveListClassName = (): string => {
        if (listActive) {
            return 'tm-home-viewmode-btn tm-home-viewmode-btn--right is-active';
        }
        return 'tm-home-viewmode-btn tm-home-viewmode-btn--right';
    };

    return (
        <Box className="tm-home-viewmode">
            <ButtonBase onClick={() => onChange(VIEW_MODE.GRID)} className={resolveGridClassName()} aria-label={t('home.common.gridView')}>
                <GridView className="tm-home-viewmode-icon" />
            </ButtonBase>
            <ButtonBase onClick={() => onChange(VIEW_MODE.LIST)} className={resolveListClassName()} aria-label={t('home.common.listView')}>
                <ViewList className="tm-home-viewmode-icon" />
            </ButtonBase>
        </Box>
    );
}
