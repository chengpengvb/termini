import {MergeCompare} from './MergeCompare.tsx';
import type {MergeEntityProps, MergeField} from '@/page/common/dto/account/MergeTypes.ts';

const FIELDS: MergeField[] = [
    {key: 'port_name', labelKey: 'account.merge.fields.serialPortName'},
    {key: 'baud_rate', labelKey: 'account.merge.fields.serialBaudRate'},
    {key: 'data_bits', labelKey: 'account.merge.fields.serialDataBits'},
    {key: 'stop_bits', labelKey: 'account.merge.fields.serialStopBits'},
    {key: 'parity', labelKey: 'account.merge.fields.serialParity'},
    {key: 'flow_control', labelKey: 'account.merge.fields.serialFlowControl'},
];

export function ConfigSerialMerge(props: MergeEntityProps) {
    return <MergeCompare fields={FIELDS} local={props.local} cloud={props.cloud}/>;
}
