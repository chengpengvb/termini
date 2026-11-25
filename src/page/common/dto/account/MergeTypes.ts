import type {ComponentType} from 'react';

export interface MergeField {
    key: string;
    labelKey: string;
}

export interface MergeEntityProps {
    local: Record<string, unknown>;
    cloud: Record<string, unknown>;
}

export interface MergeEntry {
    labelKey: string;
    component: ComponentType<MergeEntityProps>;
}
