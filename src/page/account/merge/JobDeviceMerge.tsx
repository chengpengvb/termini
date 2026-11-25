import {MergeCompare} from './MergeCompare.tsx';
import type {MergeEntityProps, MergeField} from '@/page/common/dto/account/MergeTypes.ts';

const FIELDS: MergeField[] = [
    {key: 'device_id', labelKey: 'account.merge.fields.deviceId'},
    {key: 'enabled', labelKey: 'account.merge.fields.enabled'},
    {key: 'updated_at', labelKey: 'account.merge.fields.updatedAt'},
];

export function JobDeviceMerge(props: MergeEntityProps) {
    return <MergeCompare fields={FIELDS} local={props.local} cloud={props.cloud}/>;
}
