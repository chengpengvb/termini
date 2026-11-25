import {MergeCompare} from './MergeCompare.tsx';
import type {MergeEntityProps, MergeField} from '@/page/common/dto/account/MergeTypes.ts';

const FIELDS: MergeField[] = [
    {key: 'name', labelKey: 'account.merge.fields.name'},
    {key: 'host', labelKey: 'account.merge.fields.host'},
    {key: 'type', labelKey: 'account.merge.fields.type'},
    {key: 'os_type', labelKey: 'account.merge.fields.osType'},
];

export function DeviceMerge(props: MergeEntityProps) {
    return <MergeCompare fields={FIELDS} local={props.local} cloud={props.cloud}/>;
}
