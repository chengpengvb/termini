import {MergeCompare} from './MergeCompare.tsx';
import type {MergeEntityProps, MergeField} from '@/page/common/dto/account/MergeTypes.ts';

const FIELDS: MergeField[] = [
    {key: 'base', labelKey: 'account.merge.fields.base'},
    {key: 'security', labelKey: 'account.merge.fields.security'},
    {key: 'device_id', labelKey: 'account.merge.fields.deviceId'},
    {key: 'ignore_cert', labelKey: 'account.merge.fields.ignoreCert'},
];

export function ConfigFtpMerge(props: MergeEntityProps) {
    return <MergeCompare fields={FIELDS} local={props.local} cloud={props.cloud}/>;
}
