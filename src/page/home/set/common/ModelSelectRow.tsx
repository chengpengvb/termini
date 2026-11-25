import type {ReactNode} from 'react';
import {Box, Typography} from '@mui/material';
import type {AiProvide} from '@/common/model/AiProvide.ts';
import {isEmpty, isNotEmpty} from '@/common/util/StringUtil.ts';
import {ModelSelect} from '@/page/common/components/model/ModelSelect.tsx';

export function ModelSelectRow(props: { label: string; hint: ReactNode; value: string; onChange: (value: string) => void }) {
    const {label, hint, value, onChange} = props;

    function handleSelect(provide: AiProvide): void {
        onChange(isNotEmpty(provide.id) ? provide.id : '');
    }

    return (
        <Box className="tm-set-row">
            <Box className="tm-set-row-main">
                <Typography className="tm-set-row-label">{label}</Typography>
                {hint}
            </Box>
            <Box className="tm-set-row-control">
                <ModelSelect variant="row" selectedId={isEmpty(value) ? null : value} onSelect={handleSelect}/>
            </Box>
        </Box>
    );
}
