import type {PointerEvent, RefObject} from 'react';
import {SftpHandle} from '@/page/common/dto/sftp/SftpHandle.ts';

export interface DragState {
    append: boolean;
    startViewX: number;
    startViewY: number;
    startContentY: number;
    currentViewX: number;
    currentViewY: number;
    currentContentY: number;
    visualNames: Set<string>;
}

export interface DragFromSelection {
    clientX: number;
    clientY: number;
    count: number;
}

export interface SftpDragPreviewHandle {
    hide(): void;
    move(clientX: number, clientY: number): void;
    show(clientX: number, clientY: number, count: number, targetPath: string | undefined): void;
}

export interface SftpDragSession {
    startX: number;
    startY: number;
    append: boolean;
    active: boolean;
    pointerId: number;
    startPanel: number;
    osDragStarted: boolean;
    fromSelection: boolean;
    inTransferMode: boolean;
}

export interface SftpDragOptions {
    leftRef: RefObject<SftpHandle | null>;
    rightRef: RefObject<SftpHandle | null>;
    previewRef: RefObject<SftpDragPreviewHandle | null>;
}

export interface SftpDragHandle {
    handlePointerDown(event: PointerEvent<HTMLDivElement>): void;
    handlePointerMove(event: PointerEvent<HTMLDivElement>): void;
    handlePointerUp(event: PointerEvent<HTMLDivElement>): void;
}
