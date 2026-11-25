import {MergeCompare} from './MergeCompare.tsx';
import type {MergeEntityProps, MergeField} from '@/page/common/dto/account/MergeTypes.ts';

const FIELDS: MergeField[] = [
    {key: 'device_id', labelKey: 'account.merge.fields.deviceId'},
    {key: 'config_key_pair_id', labelKey: 'account.merge.fields.configKeyPairId'},
    {key: 'share_session', labelKey: 'account.merge.fields.shareSession'},
    {key: 'preferred_auth_method', labelKey: 'account.merge.fields.preferredAuthMethod'},
    {key: 'shell_idle_timeout_seconds', labelKey: 'account.merge.fields.shellIdleTimeoutSeconds'},
    {key: 'base', labelKey: 'account.merge.fields.base'},
];

export function ConfigSshMerge(props: MergeEntityProps) {
    return <MergeCompare fields={FIELDS} local={props.local} cloud={props.cloud}/>;
}
