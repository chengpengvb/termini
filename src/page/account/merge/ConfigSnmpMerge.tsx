import {MergeCompare} from './MergeCompare.tsx';
import type {MergeEntityProps, MergeField} from '@/page/common/dto/account/MergeTypes.ts';

const FIELDS: MergeField[] = [
    {key: 'device_id', labelKey: 'account.merge.fields.deviceId'},
    {key: 'enabled', labelKey: 'account.merge.fields.snmpEnabled'},
    {key: 'version', labelKey: 'account.merge.fields.snmpVersion'},
    {key: 'port', labelKey: 'account.merge.fields.snmpPort'},
    {key: 'timeout_seconds', labelKey: 'account.merge.fields.snmpTimeout'},
    {key: 'retries', labelKey: 'account.merge.fields.snmpRetries'},
];

export function ConfigSnmpMerge(props: MergeEntityProps) {
    return <MergeCompare fields={FIELDS} local={props.local} cloud={props.cloud}/>;
}
