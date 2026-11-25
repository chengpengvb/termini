import {MergeCompare} from './MergeCompare.tsx';
import type {MergeEntityProps, MergeField} from '@/page/common/dto/account/MergeTypes.ts';

const FIELDS: MergeField[] = [
    {key: 'name', labelKey: 'account.merge.fields.name'},
    {key: 'kind', labelKey: 'account.merge.fields.forwardKind'},
    {key: 'enabled', labelKey: 'account.merge.fields.enabled'},
    {key: 'device_id', labelKey: 'account.merge.fields.deviceId'},
    {key: 'bind_address', labelKey: 'account.merge.fields.bindAddress'},
    {key: 'bind_port', labelKey: 'account.merge.fields.bindPort'},
    {key: 'target_host', labelKey: 'account.merge.fields.targetHost'},
    {key: 'target_port', labelKey: 'account.merge.fields.targetPort'},
];

export function PortForwardMerge(props: MergeEntityProps) {
    return <MergeCompare fields={FIELDS} local={props.local} cloud={props.cloud}/>;
}
