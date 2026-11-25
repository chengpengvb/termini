import {MergeCompare} from './MergeCompare.tsx';
import type {MergeEntityProps, MergeField} from '@/page/common/dto/account/MergeTypes.ts';

const FIELDS: MergeField[] = [
    {key: 'host', labelKey: 'account.merge.fields.host'},
    {key: 'port', labelKey: 'account.merge.fields.port'},
    {key: 'algorithm', labelKey: 'account.merge.fields.algorithm'},
    {key: 'sha256', labelKey: 'account.merge.fields.sha256'},
    {key: 'trusted_at', labelKey: 'account.merge.fields.trustedAt'},
];

export function KnownHostMerge(props: MergeEntityProps) {
    return <MergeCompare fields={FIELDS} local={props.local} cloud={props.cloud}/>;
}
