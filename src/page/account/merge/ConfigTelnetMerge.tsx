import {MergeCompare} from './MergeCompare.tsx';
import type {MergeEntityProps, MergeField} from '@/page/common/dto/account/MergeTypes.ts';

const FIELDS: MergeField[] = [
    {key: 'device_id', labelKey: 'account.merge.fields.deviceId'},
    {key: 'max_sessions', labelKey: 'account.merge.fields.maxSessions'},
    {key: 'nvt_eol', labelKey: 'account.merge.fields.nvtEol'},
    {key: 'username_pattern', labelKey: 'account.merge.fields.usernamePattern'},
    {key: 'password_pattern', labelKey: 'account.merge.fields.passwordPattern'},
    {key: 'base', labelKey: 'account.merge.fields.base'},
];

export function ConfigTelnetMerge(props: MergeEntityProps) {
    return <MergeCompare fields={FIELDS} local={props.local} cloud={props.cloud}/>;
}
