import {MergeCompare} from './MergeCompare.tsx';
import type {MergeEntityProps, MergeField} from '@/page/common/dto/account/MergeTypes.ts';

const FIELDS: MergeField[] = [
    {key: 'name', labelKey: 'account.merge.fields.name'},
    {key: 'public_key', labelKey: 'account.merge.fields.publicKey'},
    {key: 'certificate', labelKey: 'account.merge.fields.certificate'},
];

export function ConfigKeyPairMerge(props: MergeEntityProps) {
    return <MergeCompare fields={FIELDS} local={props.local} cloud={props.cloud}/>;
}
