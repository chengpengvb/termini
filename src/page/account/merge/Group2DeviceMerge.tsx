import {MergeCompare} from './MergeCompare.tsx';
import type {MergeEntityProps, MergeField} from '@/page/common/dto/account/MergeTypes.ts';

const FIELDS: MergeField[] = [
    {key: 'device_id', labelKey: 'account.merge.fields.deviceId'},
    {key: 'group_id', labelKey: 'account.merge.fields.groupId'},
];

export function Group2DeviceMerge(props: MergeEntityProps) {
    return <MergeCompare fields={FIELDS} local={props.local} cloud={props.cloud}/>;
}
