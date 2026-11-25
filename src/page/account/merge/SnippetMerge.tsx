import {MergeCompare} from './MergeCompare.tsx';
import type {MergeEntityProps, MergeField} from '@/page/common/dto/account/MergeTypes.ts';

const FIELDS: MergeField[] = [
    {key: 'name', labelKey: 'account.merge.fields.name'},
    {key: 'command', labelKey: 'account.merge.fields.command'},
    {key: 'remark', labelKey: 'account.merge.fields.remark'},
    {key: 'sort', labelKey: 'account.merge.fields.sort'},
];

export function SnippetMerge(props: MergeEntityProps) {
    return <MergeCompare fields={FIELDS} local={props.local} cloud={props.cloud}/>;
}
