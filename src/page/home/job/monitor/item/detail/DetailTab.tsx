import {useEffect, useState} from 'react';
import {Box, Typography} from '@mui/material';
import {ItemAlert} from './item/ItemAlert.tsx';
import {ItemChart} from './item/ItemChart.tsx';
import {ItemParse} from './item/ItemParse.tsx';
import {ItemAction} from './item/ItemAction.tsx';
import {isNil} from '@/common/util/StringUtil.ts';
import {ItemExecute} from './item/ItemExecute.tsx';
import type {JobIndicator} from '@/common/model/JobIndicator.ts';
import type {JobDevice, JobDevice2Indicator} from '@/common/model/JobDevice.ts';

export interface DetailTabProps {
    jobDevice: JobDevice;
    onDeleted: () => void;
    link: JobDevice2Indicator;
    onSaved: (link: JobDevice2Indicator) => void;
}

export function DetailTab(props: DetailTabProps) {
    const {jobDevice, link, onSaved, onDeleted} = props;
    const [error, setError] = useState<string | null>(null);
    const [intervalSecs, setIntervalSecs] = useState(link.interval_secs);
    const [draft, setDraft] = useState<JobIndicator | null>(() => (isNil(link.job_indicator) ? null : {...link.job_indicator}));

    const handlePatch = (next: Partial<JobIndicator>): void => {
        setDraft(prev => (prev ? {...prev, ...next} : prev));
    };

    useEffect(() => {
        setError(null);
        setIntervalSecs(link.interval_secs);
        setDraft(isNil(link.job_indicator) ? null : {...link.job_indicator});
    }, [link]);

    if (isNil(draft)) {
        return null;
    }

    return (
        <Box className="tm-job-drawer-pane">
            <Box className="tm-job-drawer-scroll">
                <ItemChart jobDeviceId={jobDevice.id} link={link} onError={setError}/>
                <ItemExecute draft={draft} intervalSecs={intervalSecs} onPatch={handlePatch} onIntervalChange={setIntervalSecs}/>
                <ItemParse draft={draft} onPatch={handlePatch}/>
                <ItemAlert draft={draft} onPatch={handlePatch}/>
                {error != null ? <Typography className="tm-job-drawer-error">{error}</Typography> : null}
            </Box>
            <ItemAction link={link} draft={draft} jobDeviceId={jobDevice.id} intervalSecs={intervalSecs} onSaved={onSaved} onDeleted={onDeleted} onError={setError}/>
        </Box>
    );
}
