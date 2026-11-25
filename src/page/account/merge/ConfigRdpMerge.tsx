import {MergeCompare} from './MergeCompare.tsx';
import type {MergeEntityProps, MergeField} from '@/page/common/dto/account/MergeTypes.ts';

const FIELDS: MergeField[] = [
    {key: 'base', labelKey: 'account.merge.fields.base'},
    {key: 'domain', labelKey: 'account.merge.fields.domain'},
    {key: 'security', labelKey: 'account.merge.fields.security'},
    {key: 'device_id', labelKey: 'account.merge.fields.deviceId'},
    {key: 'color_depth', labelKey: 'account.merge.fields.colorDepth'},
    {key: 'keyboard_layout', labelKey: 'account.merge.fields.keyboardLayout'},
];

export function ConfigRdpMerge(props: MergeEntityProps) {
    return <MergeCompare fields={FIELDS} local={props.local} cloud={props.cloud}/>;
}
