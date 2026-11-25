import { generateId } from "@/common/util/IdUtil";
import { PANE_NODE_KIND, SPLIT_DIRECTION, SPLIT_SIDE, type PaneConfig, type PaneLeaf, type PaneSplit } from "../core/types";

export function buildPaneTree(panes: PaneConfig[]): PaneLeaf | PaneSplit | null {
    if (panes.length === 0) {
        return null;
    }
    if (panes.length === 1) {
        return { kind: PANE_NODE_KIND.LEAF, id: generateId(), paneId: panes[0].id! };
    }
    const rest = buildPaneTree(panes.slice(1));
    const leaf: PaneLeaf = { kind: PANE_NODE_KIND.LEAF, id: generateId(), paneId: panes[0].id! };
    return {
        first: leaf,
        second: rest!,
        id: generateId(),
        ratio: 1 / panes.length,
        kind: PANE_NODE_KIND.SPLIT,
        direction: SPLIT_DIRECTION.VERTICAL,
    };
}

export function removeLeafFromTree(node: PaneLeaf | PaneSplit, paneId: string): PaneLeaf | PaneSplit | null {
    if (node.kind === PANE_NODE_KIND.LEAF) {
        return (node as PaneLeaf).paneId === paneId ? null : node;
    }
    const split = node as PaneSplit;
    const first = removeLeafFromTree(split.first, paneId);
    const second = removeLeafFromTree(split.second, paneId);
    if (first === null) {
        return second;
    }
    if (second === null) {
        return first;
    }
    if (first === split.first && second === split.second) {
        return split;
    }
    return { ...split, first, second };
}

export function insertNodeNextTo(node: PaneLeaf | PaneSplit, targetPaneId: string, newNode: PaneLeaf | PaneSplit, side: number): PaneLeaf | PaneSplit {
    if (node.kind === PANE_NODE_KIND.LEAF) {
        const leaf = node as PaneLeaf;
        if (leaf.paneId !== targetPaneId) {
            return node;
        }
        const isHorizontal = side === SPLIT_SIDE.LEFT || side === SPLIT_SIDE.RIGHT;
        const newIsFirst = side === SPLIT_SIDE.TOP || side === SPLIT_SIDE.LEFT;
        return {
            ratio: 0.5,
            id: generateId(),
            kind: PANE_NODE_KIND.SPLIT,
            first: newIsFirst ? newNode : leaf,
            second: newIsFirst ? leaf : newNode,
            direction: isHorizontal ? SPLIT_DIRECTION.HORIZONTAL : SPLIT_DIRECTION.VERTICAL,
        };
    }
    const split = node as PaneSplit;
    const first = insertNodeNextTo(split.first, targetPaneId, newNode, side);
    const second = insertNodeNextTo(split.second, targetPaneId, newNode, side);
    if (first === split.first && second === split.second) {
        return split;
    }
    return { ...split, first, second };
}

export function applyRatioToTree(node: PaneLeaf | PaneSplit, splitId: string, ratio: number): PaneLeaf | PaneSplit {
    if (node.kind === PANE_NODE_KIND.LEAF) {
        return node;
    }
    const split = node as PaneSplit;
    if (split.id === splitId) {
        return { ...split, ratio };
    }
    const first = applyRatioToTree(split.first, splitId, ratio);
    const second = applyRatioToTree(split.second, splitId, ratio);
    if (first === split.first && second === split.second) {
        return split;
    }
    return { ...split, first, second };
}
