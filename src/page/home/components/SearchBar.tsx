import {Search} from '@mui/icons-material';
import {Box, InputBase} from '@mui/material';
import {isEmpty, isNotEmpty} from '@/common/util/StringUtil';

export interface SearchBarProps {
    placeholder: string;
    searchText: string | null;
    onChange: (value: string | null) => void;
}

export function SearchBar(props: SearchBarProps) {
    const {placeholder, searchText, onChange} = props;

    const resolveInputValue = (): string | undefined => {
        if (isNotEmpty(searchText)) {
            return searchText;
        }
        return;
    };

    const handleInput = (raw: string): void => {
        if (isEmpty(raw)) {
            onChange(null);
            return;
        }
        onChange(raw);
    };

    return (
        <Box className="tm-home-searchbar">
            <Search className="tm-home-search-icon"/>
            <InputBase value={resolveInputValue()} onChange={e => handleInput(e.target.value)} placeholder={placeholder} className="tm-home-search-input-root" inputProps={{className: 'tm-home-search-input'}}/>
        </Box>
    );
}
