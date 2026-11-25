import {PopoverPaper} from '@mui/material/Popover';
import {Paper, type PaperProps} from '@mui/material';
import {createElement, forwardRef, type Ref} from 'react';
import {SheetTransition} from '@/page/common/components/dialog/SheetTransition.tsx';

interface ScopedDialogProps {
    className?: string;
    disablePortal?: boolean;
    disableScrollLock?: boolean;
    disableAutoFocus?: boolean;
    disableRestoreFocus?: boolean;
    disableEnforceFocus?: boolean;
    sx?: { position: 'absolute' };
    container?: () => HTMLElement;
    backdropSlotProps?: { sx: { position: 'absolute' } };
}

function markOverlayNode(node: HTMLDivElement | null): void {
    if (node === null) {
        return;
    }
    node.setAttribute('data-tm-overlay', 'true');
}

function bindOverlayRef(ref: Ref<HTMLDivElement> | null | undefined, node: HTMLDivElement | null): void {
    if (ref == null) {
        markOverlayNode(node);
        return;
    }
    const objectRef = ref as {current: HTMLDivElement | null};
    if (objectRef.current !== undefined) {
        objectRef.current = node;
        markOverlayNode(node);
        return;
    }
    (ref as (instance: HTMLDivElement | null) => void)(node);
    markOverlayNode(node);
}

export const OverlayPaper = forwardRef<HTMLDivElement, PaperProps>(function OverlayPaper(props, ref) {
    return createElement(Paper, {...props, ref: (node: HTMLDivElement | null) => bindOverlayRef(ref, node)});
});

export const OverlayPopoverPaper = forwardRef<HTMLDivElement, PaperProps>(function OverlayPopoverPaper(props, ref) {
    return createElement(PopoverPaper as typeof Paper, {...props, ref: (node: HTMLDivElement | null) => bindOverlayRef(ref, node)});
});

export function resolveScopedDialogProps(container: HTMLElement | null | undefined): ScopedDialogProps {
    if (!container) {
        return {};
    }
    return {
        disablePortal: false,
        disableScrollLock: true,
        disableAutoFocus: true,
        disableRestoreFocus: true,
        disableEnforceFocus: true,
        container: () => container,
        sx: {position: 'absolute'},
        className: 'tm-dialog-scoped',
        backdropSlotProps: {sx: {position: 'absolute'}},
    };
}

export function resolveSheetDialogProps(paper: { className: string; sx?: object }): {
    disableScrollLock: true;
    disableAutoFocus: true;
    disableRestoreFocus: true;
    slots: { paper: typeof OverlayPaper; transition: typeof SheetTransition };
    slotProps: { paper: { className: string; sx?: object } };
} {
    return {
        disableScrollLock: true,
        disableAutoFocus: true,
        disableRestoreFocus: true,
        slots: {
            paper: OverlayPaper,
            transition: SheetTransition,
        },
        slotProps: {
            paper,
        },
    };
}
