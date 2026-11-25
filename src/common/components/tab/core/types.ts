import type {ReactNode} from 'react';

export enum EVENT_TYPE {
    OPEN = 1,
    CLOSE = 2,
    ACTIVE = 3,
    LAYOUT = 4,
    DRAG = 10,
    DROP_LANDING = 11,
    BAR_TOOL = 20,
    SIDE_PANEL = 21,
    SIDE_PANEL_VISIBLE = 22,
    PANE_TOOL = 23,
}

export const SPLIT_DIRECTION = {
    HORIZONTAL: 1,
    VERTICAL: 2,
} as const;

export const PANE_NODE_KIND = {
    LEAF: 1,
    SPLIT: 2,
} as const;

export const SPLIT_SIDE = {
    TOP: 1,
    BOTTOM: 2,
    LEFT: 3,
    RIGHT: 4,
} as const;

export interface PaneLeaf {
    id: string;
    kind: number;
    paneId: string;
}

export interface PaneSplit {
    kind: number;
    id: string;
    ratio: number;
    direction: number;
    first: PaneLeaf | PaneSplit;
    second: PaneLeaf | PaneSplit;
}

export interface TabBar {
    title: string;
    icon?: ReactNode;
    canClose: boolean;
    canMerge?: boolean;
    actionTitle?: string;
    actionIcon?: ReactNode;
    titleClassName?: string;
    onActionClick?: () => void;
}

export interface PaneData {
    type: number;
    data?: any;
}

export interface PaneConfig {
    id?: string;
    tabBar: TabBar;
    data?: PaneData;
    content: ReactNode;
}

export interface TabConfig {
    id?: string;
    panes: PaneConfig[];
    focusedPaneId?: string;
    paneTree?: PaneLeaf | PaneSplit | null;
}

export const BAR_TOOL_SLOT = {
    RIGHT: 1,
    STRIP: 2,
} as const;

export interface BarToolBar {
    title?: string;
    slot?: number;
    icon?: ReactNode;
    onClick?: () => void;
}

export interface BarToolConfig {
    id?: string;
    tabBar: BarToolBar;
}

export interface PaneToolBar {
    title: string;
    icon: ReactNode;
    keys?: string[];
    onClick: () => void;
    active?: () => boolean;
}

export interface PaneToolConfig {
    id?: string;
    paneId: string;
    tabBar: PaneToolBar;
}

export const TAB_LAYOUT_REASON = {
    PANE_MUTATION: 1,
    SPLIT_RATIO: 2,
    FOCUS: 3,
    TAB_BAR: 4,
} as const;

export interface TabLayoutEventData {
    reason: number;
    tabs: TabConfig[];
}
