import {GroupMerge} from '@/page/account/merge/GroupMerge.tsx';
import {CLOUD_SYNC_TYPE} from '@/common/model/dto/CloudSync.ts';
import {DeviceMerge} from '@/page/account/merge/DeviceMerge.tsx';
import {AiSkillMerge} from '@/page/account/merge/AiSkillMerge.tsx';
import {SnippetMerge} from '@/page/account/merge/SnippetMerge.tsx';
import {ConfigFtpMerge} from '@/page/account/merge/ConfigFtpMerge.tsx';
import {ConfigRdpMerge} from '@/page/account/merge/ConfigRdpMerge.tsx';
import {ConfigSshMerge} from '@/page/account/merge/ConfigSshMerge.tsx';
import {JobDeviceMerge} from '@/page/account/merge/JobDeviceMerge.tsx';
import {KnownHostMerge} from '@/page/account/merge/KnownHostMerge.tsx';
import type {MergeEntry} from '@/page/common/dto/account/MergeTypes.ts';
import {ConfigSnmpMerge} from '@/page/account/merge/ConfigSnmpMerge.tsx';
import {AiMcpServerMerge} from '@/page/account/merge/AiMcpServerMerge.tsx';
import {PortForwardMerge} from '@/page/account/merge/PortForwardMerge.tsx';
import {ConfigSerialMerge} from '@/page/account/merge/ConfigSerialMerge.tsx';
import {ConfigTelnetMerge} from '@/page/account/merge/ConfigTelnetMerge.tsx';
import {Group2DeviceMerge} from '@/page/account/merge/Group2DeviceMerge.tsx';
import {JobIndicatorMerge} from '@/page/account/merge/JobIndicatorMerge.tsx';
import {ConfigKeyPairMerge} from '@/page/account/merge/ConfigKeyPairMerge.tsx';
import {JobDevice2IndicatorMerge} from '@/page/account/merge/JobDevice2IndicatorMerge.tsx';

export const MERGE_REGISTRY: Record<number, MergeEntry> = {
    [CLOUD_SYNC_TYPE.DEVICE]: {labelKey: 'account.merge.types.device', component: DeviceMerge},
    [CLOUD_SYNC_TYPE.GROUP]: {labelKey: 'account.merge.types.group', component: GroupMerge},
    [CLOUD_SYNC_TYPE.GROUP_2_DEVICE]: {labelKey: 'account.merge.types.group2Device', component: Group2DeviceMerge},
    [CLOUD_SYNC_TYPE.CONFIG_SSH]: {labelKey: 'account.merge.types.configSsh', component: ConfigSshMerge},
    [CLOUD_SYNC_TYPE.CONFIG_TELNET]: {labelKey: 'account.merge.types.configTelnet', component: ConfigTelnetMerge},
    [CLOUD_SYNC_TYPE.CONFIG_RDP]: {labelKey: 'account.merge.types.configRdp', component: ConfigRdpMerge},
    [CLOUD_SYNC_TYPE.KNOWN_HOST]: {labelKey: 'account.merge.types.knownHost', component: KnownHostMerge},
    [CLOUD_SYNC_TYPE.CONFIG_KEY_PAIR]: {labelKey: 'account.merge.types.configKeyPair', component: ConfigKeyPairMerge},
    [CLOUD_SYNC_TYPE.PORT_FORWARD]: {labelKey: 'account.merge.types.portForward', component: PortForwardMerge},
    [CLOUD_SYNC_TYPE.JOB_DEVICE]: {labelKey: 'account.merge.types.jobDevice', component: JobDeviceMerge},
    [CLOUD_SYNC_TYPE.JOB_INDICATOR]: {labelKey: 'account.merge.types.jobIndicator', component: JobIndicatorMerge},
    [CLOUD_SYNC_TYPE.JOB_DEVICE_2_INDICATOR]: {labelKey: 'account.merge.types.jobDevice2Indicator', component: JobDevice2IndicatorMerge},
    [CLOUD_SYNC_TYPE.AI_SKILL]: {labelKey: 'account.merge.types.aiSkill', component: AiSkillMerge},
    [CLOUD_SYNC_TYPE.AI_MCP_SERVER]: {labelKey: 'account.merge.types.aiMcpServer', component: AiMcpServerMerge},
    [CLOUD_SYNC_TYPE.CONFIG_SNMP]: {labelKey: 'account.merge.types.configSnmp', component: ConfigSnmpMerge},
    [CLOUD_SYNC_TYPE.SNIPPET]: {labelKey: 'account.merge.types.snippet', component: SnippetMerge},
    [CLOUD_SYNC_TYPE.CONFIG_SERIAL]: {labelKey: 'account.merge.types.configSerial', component: ConfigSerialMerge},
    [CLOUD_SYNC_TYPE.CONFIG_FTP]: {labelKey: 'account.merge.types.configFtp', component: ConfigFtpMerge},
};

export function resolveMergeEntry(syncType: number): MergeEntry {
    const entry = MERGE_REGISTRY[syncType];
    if (!entry) {
        throw new Error(`unknown cloud sync type: ${syncType}`);
    }
    return entry;
}
