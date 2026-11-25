import {MergeCompare} from './MergeCompare.tsx';
import type {MergeEntityProps, MergeField} from '@/page/common/dto/account/MergeTypes.ts';

const FIELDS: MergeField[] = [
    {key: 'url', labelKey: 'account.merge.fields.url'},
    {key: 'env', labelKey: 'account.merge.fields.env'},
    {key: 'args', labelKey: 'account.merge.fields.args'},
    {key: 'name', labelKey: 'account.merge.fields.serverName'},
    {key: 'enabled', labelKey: 'account.merge.fields.enabled'},
    {key: 'command', labelKey: 'account.merge.fields.command'},
    {key: 'headers', labelKey: 'account.merge.fields.headers'},
    {key: 'transport', labelKey: 'account.merge.fields.transport'},
    {key: 'timeout_secs', labelKey: 'account.merge.fields.timeoutSecs'},
];

export function AiMcpServerMerge(props: MergeEntityProps) {
    return <MergeCompare fields={FIELDS} local={props.local} cloud={props.cloud}/>;
}
