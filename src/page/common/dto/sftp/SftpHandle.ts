import type { DragFromSelection } from '@/page/common/dto/sftp/SftpDrag.ts';

export interface SftpHandle {
    isLocal(): boolean;
    openSearch(): void;
    getSessionId(): string;
    showDragCover(): void;
    hideDragCover(): void;
    clearSelection(): void;
    cancelDragSelect(): void;
    getSelectedNames(): string[];
    getCurrentPath(): string | undefined;
    getRootElement(): HTMLElement | null;
    getClientTransferId(): string | undefined;
    setFilterQuery(query: string | null): void;
    endDragSelect(clientX: number, clientY: number): void;
    openActionsMenu(clientX: number, clientY: number): void;
    updateDragSelect(clientX: number, clientY: number): void;
    beginDragSelect(clientX: number, clientY: number, append: boolean, onFromSelection?: (ctx: DragFromSelection) => void): void;

}
