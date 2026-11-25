import {MergeCompare} from './MergeCompare.tsx';
import type {MergeEntityProps, MergeField} from '@/page/common/dto/account/MergeTypes.ts';

const FIELDS: MergeField[] = [
    {key: 'job_device_id', labelKey: 'account.merge.fields.jobDeviceId'},
    {key: 'job_indicator_id', labelKey: 'account.merge.fields.jobIndicatorId'},
    {key: 'interval_secs', labelKey: 'account.merge.fields.intervalSecs'},
];

export function JobDevice2IndicatorMerge(props: MergeEntityProps) {
    return <MergeCompare fields={FIELDS} local={props.local} cloud={props.cloud}/>;
}
