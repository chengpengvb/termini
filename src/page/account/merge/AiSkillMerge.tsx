import {MergeCompare} from './MergeCompare.tsx';
import type {MergeEntityProps, MergeField} from '@/page/common/dto/account/MergeTypes.ts';

const FIELDS: MergeField[] = [
    {key: 'name', labelKey: 'account.merge.fields.name'},
    {key: 'description', labelKey: 'account.merge.fields.skillDescription'},
    {key: 'sort', labelKey: 'account.merge.fields.sort'},
    {key: 'content', labelKey: 'account.merge.fields.content'},
];

export function AiSkillMerge(props: MergeEntityProps) {
    return <MergeCompare fields={FIELDS} local={props.local} cloud={props.cloud}/>;
}
