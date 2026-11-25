import {DragFromSelection} from "@/page/common/dto/sftp/SftpDrag.ts";

export interface BodyEntriesHandle {
    openSearch(): void;
    getScrollTop(): number;
    jumpToIndex(index: number): void;
    setFilterQuery(query: string | null): void;
    setSearchQuery(query: string | null): void;
    openActionsMenu(clientX: number, clientY: number): void;
}

export interface BodyEntriesVirtualHandle {
    getScrollTop(): number;
    clearSelection(): void;
    cancelDragSelect(): void;
    getSelectedNames(): string[];
    scrollToIndex(index: number): void;
    setScrollTop(scrollTop: number): void;
    setSearchQuery(query: string | null): void;
    endDragSelect(clientX: number, clientY: number): void;
    updateDragSelect(clientX: number, clientY: number): void;
    beginDragSelect(clientX: number, clientY: number, append: boolean, onFromSelection?: (ctx: DragFromSelection) => void): void;
}
