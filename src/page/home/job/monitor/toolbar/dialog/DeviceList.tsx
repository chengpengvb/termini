import {useMemo, useState} from 'react';
import {useI18n} from '@/common/i18n/useI18n.ts';
import type {Device} from '@/common/model/Device.ts';
import {CheckBox, CheckBoxOutlineBlank, IndeterminateCheckBox, Search} from '@mui/icons-material';
import {Box, ButtonBase, InputBase, Table, TableBody, TableCell, TableHead, TableRow, Typography} from '@mui/material';

interface DeviceListProps {
    candidates: Device[];
    selectedIds: string[];
    onToggle: (id: string) => void;
    onChangeSelectedIds: (ids: string[]) => void;
}

function matchDevice(device: Device, keyword: string, osLabel: string): boolean {
    if (device.name.toLowerCase().includes(keyword)) {
        return true;
    }
    if (device.host.toLowerCase().includes(keyword)) {
        return true;
    }
    return osLabel.toLowerCase().includes(keyword);
}

export function DeviceList(props: DeviceListProps) {
    const {t, tf} = useI18n();
    const [query, setQuery] = useState('');
    const keyword = query.trim().toLowerCase();
    const {candidates, selectedIds, onToggle, onChangeSelectedIds} = props;
    const osLabelByType = useMemo(() => {
        const map = new Map<number, string>();
        for (const device of candidates) {
            if (!map.has(device.os_type)) {
                map.set(device.os_type, t(`home.devices.osType.${device.os_type}`));
            }
        }
        return map;
    }, [candidates, t]);
    const filtered = keyword ? candidates.filter(device => matchDevice(device, keyword, osLabelByType.get(device.os_type) ?? '')) : candidates;
    const filteredIds = filtered.map(device => device.id);
    const selectedInView = filteredIds.filter(id => selectedIds.includes(id));
    const allSelected = filteredIds.length > 0 && selectedInView.length === filteredIds.length;
    const someSelected = selectedInView.length > 0 && !allSelected;

    const toggleAll = (): void => {
        if (allSelected) {
            const drop = new Set(filteredIds);
            onChangeSelectedIds(selectedIds.filter(id => !drop.has(id)));
            return;
        }
        onChangeSelectedIds([...new Set([...selectedIds, ...filteredIds])]);
    };

    const renderHeadCheck = () => {
        if (allSelected) {
            return <CheckBox className="tm-job-table-check"/>;
        }
        if (someSelected) {
            return <IndeterminateCheckBox className="tm-job-table-check"/>;
        }
        return <CheckBoxOutlineBlank className="tm-job-table-check"/>;
    };

    return (
        <>
            <Box className="tm-home-searchbar tm-job-dialog-search">
                <Search className="tm-home-search-icon"/>
                <InputBase className="tm-home-search-input-root" inputProps={{className: 'tm-home-search-input'}} placeholder={t('home.job.deviceListSearch')} value={query} onChange={event => setQuery(event.target.value)}/>
            </Box>
            <Typography className="tm-job-muted">
                {tf('home.job.deviceListSummaryFiltered', {total: candidates.length, filtered: filtered.length, selected: selectedIds.length})}
            </Typography>
            <Box className="tm-job-table-wrap">
                {filtered.length === 0 ? (
                    <Typography className="tm-job-muted" sx={{p: 1.5}}>{t('home.job.deviceListNoMatch')}</Typography>
                ) : (
                    <Table stickyHeader size="small" className="tm-job-device-table">
                        <TableHead>
                            <TableRow>
                                <TableCell className="tm-job-table-col-check is-select-all">
                                    <ButtonBase disableRipple className="tm-job-table-select-all" onClick={toggleAll} aria-label={allSelected ? t('home.job.deviceListDeselectFiltered') : t('home.job.deviceListSelectFiltered')}>
                                        {renderHeadCheck()}
                                        <Typography component="span" className="tm-job-table-select-all-label">{allSelected ? t('home.common.deselectAll') : t('home.common.selectAll')}</Typography>
                                    </ButtonBase>
                                </TableCell>
                                <TableCell className="tm-job-table-col-name">{t('home.common.name')}</TableCell>
                                <TableCell className="tm-job-table-col-host">{t('home.common.host')}</TableCell>
                                <TableCell className="tm-job-table-col-type">{t('home.common.type')}</TableCell>
                            </TableRow>
                        </TableHead>
                        <TableBody>
                            {filtered.map(device => {
                                const selected = selectedIds.includes(device.id);
                                return (
                                    <TableRow key={device.id} hover selected={selected} className={selected ? 'tm-job-table-row is-selected' : 'tm-job-table-row'} onClick={() => onToggle(device.id)}>
                                        <TableCell className="tm-job-table-col-check">
                                            <Box className="tm-job-table-check-slot">
                                                {selected ? <CheckBox className="tm-job-table-check"/> : <CheckBoxOutlineBlank className="tm-job-table-check"/>}
                                            </Box>
                                        </TableCell>
                                        <TableCell className="tm-job-table-col-name">{device.name}</TableCell>
                                        <TableCell className="tm-job-table-col-host">{device.host}</TableCell>
                                        <TableCell className="tm-job-table-col-type">{t(`home.devices.osType.${device.os_type}`)}</TableCell>
                                    </TableRow>
                                );
                            })}
                        </TableBody>
                    </Table>
                )}
            </Box>
        </>
    );
}
