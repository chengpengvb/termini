import {useI18n} from '@/common/i18n/useI18n.ts';
import {isNil} from '@/common/util/StringUtil.ts';
import {formatTime, type JobTone} from '@/page/common/util/jobUtil.ts';
import {Box, ButtonBase, CircularProgress, Typography} from '@mui/material';
import {JOB_DISCOVER_STATUS, isDiscoverRunning, type JobDiscover} from '@/common/model/JobDiscover.ts';
import {CheckBox, CheckBoxOutlineBlank, DeleteOutlined, IndeterminateCheckBox} from '@mui/icons-material';

export interface DiscoverRow {
    host: string;
    deviceName: string;
    discover: JobDiscover;
}

interface DiscoverListProps {
    rows: DiscoverRow[];
    checkedIds: string[];
    selectedId: string | null;
    onToggleAll: () => void;
    onToggle: (id: string) => void;
    onDelete: (id: string) => void;
    onSelect: (row: DiscoverRow) => void;
}

function toneOf(status: number): JobTone {
    if (status === JOB_DISCOVER_STATUS.SUCCESS) {
        return 'ok';
    }
    if (status === JOB_DISCOVER_STATUS.FAILED) {
        return 'firing';
    }
    return 'neutral';
}

export function DiscoverList(props: DiscoverListProps) {
    const {t, tf} = useI18n();
    const {rows, selectedId, checkedIds, onToggle, onToggleAll, onDelete, onSelect} = props;
    const allSelected = rows.length > 0 && checkedIds.length === rows.length;
    const someSelected = checkedIds.length > 0 && !allSelected;

    const resolveDiscoverStatusLabel = (status: number): string => {
        return t(`home.job.discover.discoverStatus.${status}`);
    };

    const formatAgoLabel = (millis: number | null | undefined): string => {
        if (isNil(millis) || millis <= 0) {
            return t('home.common.emDash');
        }
        const diff = Math.max(0, Date.now() - millis);
        if (diff < 60_000) {
            return t('home.job.ago.justNow');
        }
        if (diff < 3600_000) {
            return tf('home.job.ago.minutes', {n: Math.floor(diff / 60_000)});
        }
        if (diff < 24 * 3600_000) {
            return tf('home.job.ago.hours', {n: Math.floor(diff / 3600_000)});
        }
        if (diff < 30 * 24 * 3600_000) {
            return tf('home.job.ago.days', {n: Math.floor(diff / (24 * 3600_000))});
        }
        if (diff < 365 * 24 * 3600_000) {
            return tf('home.job.ago.months', {n: Math.floor(diff / (30 * 24 * 3600_000))});
        }
        return tf('home.job.ago.years', {n: Math.floor(diff / (365 * 24 * 3600_000))});
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

    if (rows.length === 0) {
        return (
            <Box className="tm-job-alarm-empty">
                <Typography className="tm-job-muted">{t('home.job.discover.empty')}</Typography>
            </Box>
        );
    }

    return (
        <Box className="tm-job-alarm-list tm-job-discover-list">
            <ButtonBase disableRipple className="tm-job-alarm-list-bar" onClick={onToggleAll} aria-label={allSelected ? t('home.common.deselectAll') : t('home.common.selectAll')}>
                <Box className="tm-job-table-check-slot">{renderHeadCheck()}</Box>
                <Typography className="tm-job-alarm-list-bar-label">{allSelected ? t('home.common.deselectAll') : t('home.common.selectAll')}</Typography>
            </ButtonBase>
            {rows.map(row => {
                const discover = row.discover;
                const tone = toneOf(discover.status);
                const selected = selectedId === discover.id;
                const checked = checkedIds.includes(discover.id);
                const running = isDiscoverRunning(discover.status);
                const rowClass = [
                    'tm-device-item',
                    'tm-job-discover-row',
                    `is-${tone}`,
                    selected ? 'is-selected' : null,
                    checked ? 'is-checked' : null,
                ].filter(Boolean).join(' ');
                return (
                    <ButtonBase key={discover.id} component="div" disableRipple className={rowClass} onClick={() => onSelect(row)}>
                        <Box className="tm-job-table-check-btn" role="button" aria-label={checked ? t('home.common.deselect') : t('home.common.select')} onClick={event => { event.stopPropagation(); onToggle(discover.id); }}>
                            {checked ? <CheckBox className="tm-job-table-check"/> : <CheckBoxOutlineBlank className="tm-job-table-check"/>}
                        </Box>
                        <Box className="tm-device-item-icon" aria-hidden>
                            {running ? (
                                <CircularProgress size={13} thickness={5} className="tm-job-device-explore-spin"/>
                            ) : (
                                <Box className={`tm-job-status-dot is-${tone}`}/>
                            )}
                        </Box>
                        <Box className="tm-device-item-info">
                            <Typography className="tm-device-item-name tm-ellipsis">{row.deviceName}</Typography>
                            <Typography className="tm-device-item-ip tm-ellipsis" title={formatTime(discover.timestamp)}>
                                {resolveDiscoverStatusLabel(discover.status)} · {row.host} · {formatAgoLabel(discover.timestamp)}
                            </Typography>
                        </Box>
                        <Box className="tm-device-item-actions">
                            <Box className="tm-job-icon-btn tm-job-discover-row-delete" role="button" aria-label={t('common.delete')} title={t('common.delete')} onClick={event => { event.stopPropagation(); onDelete(discover.id); }}>
                                <DeleteOutlined sx={{fontSize: 16}}/>
                            </Box>
                        </Box>
                    </ButtonBase>
                );
            })}
        </Box>
    );
}
