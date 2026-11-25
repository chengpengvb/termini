import {translate} from '@/common/i18n/i18n.ts';

export const DEFAULT_GROUP_ID = '000000';

export const NEW_GROUP_ACTION_ID = '11111111111';

export interface Group {
    id: string;
    name: string;
}

export function resolveGroupName(group: Group): string {
    if (group.id === DEFAULT_GROUP_ID) {
        return translate('home.devices.defaultGroup');
    }
    return group.name;
}
