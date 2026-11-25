import {useState} from 'react';
import type {MouseEvent, ReactNode} from 'react';
import type {SelectChangeEvent} from '@mui/material';
import {ArrowDropDown, Check} from '@mui/icons-material';
import {isNotEmpty, isNotNil} from "@/common/util/StringUtil.ts";
import {usePlatformMobile} from '@/common/context/usePlatformMobile';
import {Box, MenuItem, MenuList, Select, Typography} from '@mui/material';
import {ActionSheet} from '@/page/common/components/dialog/ActionSheet.tsx';

interface SelectOption {
    value: string;
    label: string;
}

export function MultiSelectField(props: { label?: string; value: string[]; onChange: (value: string[]) => void; options: SelectOption[]; labelWidth?: number; placeholder?: string }) {
    const mobileViewport = usePlatformMobile();
    const [anchorEl, setAnchorEl] = useState<HTMLElement | null>(null);
    const {label, value, onChange, options, labelWidth, placeholder} = props;
    const open = Boolean(anchorEl);

    const resolveLabelWidth = (): number => {
        if (isNotNil(labelWidth)) {
            return labelWidth;
        }
        return 64;
    };

    const handleChange = (e: SelectChangeEvent<string[]>): void => {
        const raw = e.target.value;
        if (Array.isArray(raw)) {
            onChange(raw);
            return;
        }
        onChange(raw.split(','));
    };

    const resolveOptionLabel = (item: string): string => {
        const found = options.find(opt => opt.value === item);
        if (isNotNil(found)) {
            return found.label;
        }
        return item;
    };

    const renderSelectedValue = (v: string[]): ReactNode => {
        if (v.length === 0) {
            if (isNotEmpty(placeholder)) {
                return <Box component="span" sx={{color: 'var(--tm-text-tertiary)', opacity: 0.7}}>{placeholder}</Box>;
            }
            return '';
        }
        return v.map(resolveOptionLabel).join(', ');
    };

    const isOptionChecked = (optValue: string): boolean => {
        return value.includes(optValue);
    };

    const handleOpen = (e: MouseEvent<HTMLElement>): void => {
        setAnchorEl(e.currentTarget);
    };

    const handleClose = (): void => {
        setAnchorEl(null);
    };

    const handleToggle = (optValue: string): void => {
        if (value.includes(optValue)) {
            onChange(value.filter(item => item !== optValue));
            return;
        }
        onChange([...value, optValue]);
    };

    if (mobileViewport) {
        return (
            <Box>
                <Box className="tm-edit-field" sx={{display: 'flex', alignItems: 'center', minHeight: '38px', border: '1px solid var(--tm-border-subtle)', borderRadius: '7px', bgcolor: 'transparent', transition: 'border-color 120ms ease, background-color 120ms ease', '&:hover': {bgcolor: 'var(--tm-bg-hover)'}, '&:focus-within': {bgcolor: 'var(--tm-bg-hover)', borderColor: 'var(--tm-border-accent)'}}} onClick={handleOpen}>
                    {isNotNil(label) ? (
                        <Typography className="tm-edit-field-label" sx={{fontSize: 13, color: 'var(--tm-text-tertiary)', pl: 1, width: resolveLabelWidth(), flexShrink: 0, whiteSpace: 'nowrap', userSelect: 'none'}}>{label}</Typography>
                    ) : null}
                    <Box className="tm-edit-field-value" sx={{flex: 1, minWidth: 0, px: 1, py: 0.5, fontSize: 13, color: 'var(--tm-text-primary)', overflow: 'clip', textOverflow: 'ellipsis', whiteSpace: 'nowrap', cursor: 'pointer'}}>
                        {renderSelectedValue(value)}
                    </Box>
                    <ArrowDropDown className="tm-edit-field-caret"/>
                </Box>
                <ActionSheet open={open} onClose={handleClose}>
                    <MenuList disablePadding className="tm-action-sheet-list">
                        {options.map(opt => (
                            <MenuItem key={opt.value} className={isOptionChecked(opt.value) ? 'tm-action-sheet-item is-selected' : 'tm-action-sheet-item'} onClick={() => handleToggle(opt.value)}>
                                <Box component="span" className="tm-menu-item-icon">
                                    {isOptionChecked(opt.value) ? <Check/> : null}
                                </Box>
                                <Box component="span" className="tm-menu-item-text">{opt.label}</Box>
                            </MenuItem>
                        ))}
                    </MenuList>
                </ActionSheet>
            </Box>
        );
    }

    return (
        <Box className="tm-edit-field" sx={{display: 'flex', alignItems: 'center', minHeight: '38px', border: '1px solid var(--tm-border-subtle)', borderRadius: '7px', bgcolor: 'transparent', transition: 'border-color 120ms ease, background-color 120ms ease', '&:hover': {bgcolor: 'var(--tm-bg-hover)'}, '&:focus-within': {bgcolor: 'var(--tm-bg-hover)', borderColor: 'var(--tm-border-accent)'}}}>
            {isNotNil(label) ? (
                <Typography className="tm-edit-field-label" sx={{fontSize: 13, color: 'var(--tm-text-tertiary)', pl: 1, width: resolveLabelWidth(), flexShrink: 0, whiteSpace: 'nowrap', userSelect: 'none'}}>{label}</Typography>
            ) : null}
            <Select
                className="tm-edit-field-input"
                multiple
                value={value}
                onChange={handleChange}
                displayEmpty
                variant="standard"
                disableUnderline
                renderValue={renderSelectedValue}
                sx={{flex: 1, minWidth: 0, px: 1, fontSize: 13, color: 'var(--tm-text-primary)', '& .MuiSelect-select': {py: 0.5, px: 0, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis', '&:focus': {bgcolor: 'transparent'}}, '& .MuiSvgIcon-root': {color: 'var(--tm-text-tertiary)'}, '& .MuiSelect-icon': {right: '6px'}}}
                MenuProps={{
                    anchorOrigin: {vertical: 'bottom', horizontal: 'center'},
                    transformOrigin: {vertical: 'top', horizontal: 'center'},
                    slotProps: {paper: {className: 'tm-menu-paper', sx: {minWidth: 220, maxHeight: 320}}, list: {className: 'tm-menu-list'}},
                }}
            >
                {options.map(opt => (
                    <MenuItem key={opt.value} value={opt.value} className={isOptionChecked(opt.value) ? 'tm-menu-item is-selected' : 'tm-menu-item'}>
                        <Box className="tm-menu-item-icon">
                            {isOptionChecked(opt.value) ? <Check/> : null}
                        </Box>
                        <Box className="tm-menu-item-text">{opt.label}</Box>
                    </MenuItem>
                ))}
            </Select>
        </Box>
    );
}
