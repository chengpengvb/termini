import {useI18n} from '@/common/i18n/useI18n.ts';
import {isEmpty} from '@/common/util/StringUtil.ts';
import {Box, ButtonBase, Typography} from '@mui/material';
import {formatInterval} from '@/page/common/util/jobUtil.ts';
import {CheckBox, CheckBoxOutlineBlank} from '@mui/icons-material';
import {protocolLabel, ruleOpLabel} from '@/common/model/JobIndicator.ts';
import {JOB_DISCOVER_ITEM_STATUS, type JobDiscoverItem} from '@/common/model/JobDiscover.ts';

interface CandidateListProps {
    running: boolean;
    selectedIds: string[];
    items: JobDiscoverItem[];
    onToggle: (id: string) => void;
}

export function CandidateList(props: CandidateListProps) {
    const {t, tf} = useI18n();
    const {running, items, selectedIds, onToggle} = props;
    const pendingCount = items.filter(item => item.status === JOB_DISCOVER_ITEM_STATUS.CANDIDATE).length;

    const resolveItemStatusLabel = (status: number): string => {
        if (status === JOB_DISCOVER_ITEM_STATUS.ACCEPTED) {
            return t('home.job.discover.candidateAccepted');
        }
        return t('home.job.discover.candidatePending');
    };

    return (
        <Box className="tm-discover-section">
            <Typography className="tm-discover-section-title">{tf('home.job.discover.candidatesTitle', {pending: pendingCount, total: items.length})}</Typography>
            {items.length === 0 ? (
                <Typography className="tm-job-muted">{running ? t('home.job.discover.candidatesRunning') : t('home.job.discover.candidatesEmpty')}</Typography>
            ) : (
                <Box className="tm-discover-items">
                    {items.map(item => {
                        const pending = item.status === JOB_DISCOVER_ITEM_STATUS.CANDIDATE;
                        const selected = selectedIds.includes(item.id);
                        const sampleSuffix = isEmpty(item.sample_value) ? '' : tf('home.job.discover.candidateSample', {value: item.sample_value});
                        const meta = tf('home.job.discover.candidateMeta', {
                            protocol: protocolLabel(item.protocol),
                            interval: formatInterval(item.interval_secs),
                            op: ruleOpLabel(item.alert_rule_op),
                            threshold: item.alert_threshold,
                        }) + sampleSuffix;
                        return (
                            <ButtonBase key={item.id} disableRipple className={selected ? 'tm-discover-item is-selected' : 'tm-discover-item'} disabled={!pending} onClick={() => onToggle(item.id)}>
                                <Box className="tm-job-table-check-slot">
                                    {selected ? <CheckBox className="tm-job-table-check"/> : <CheckBoxOutlineBlank className="tm-job-table-check"/>}
                                </Box>
                                <Box className="tm-discover-item-main">
                                    <Typography className="tm-discover-item-name">{item.name}</Typography>
                                    <Typography className="tm-discover-item-sub">{meta}</Typography>
                                    {!isEmpty(item.reason) ? <Typography className="tm-discover-item-sub">{item.reason}</Typography> : null}
                                </Box>
                                <Typography className={pending ? 'tm-job-alarm-tag is-pending' : 'tm-job-alarm-tag is-ok'}>{resolveItemStatusLabel(item.status)}</Typography>
                            </ButtonBase>
                        );
                    })}
                </Box>
            )}
        </Box>
    );
}
