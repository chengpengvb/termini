import type {Editor} from '@tiptap/react';
import {useEffect, type RefObject} from 'react';
import {inTauri} from '@/common/util/TauriUtil.ts';
import {isEmpty, isNotNil} from '@/common/util/StringUtil.ts';
import {CHAT_TARGET_TYPE} from '@/common/const/ChatTargetConst.ts';
import {useSystemUtilService} from '@/common/context/useAppHooks.tsx';
import type {ChatReference} from '@/common/model/dto/ChatReference.ts';
import {resolveLocalPathName} from '@/page/common/util/reference/token.ts';
import {insertEditorReference} from '@/page/common/util/reference/editor.ts';
import {getCurrentWebview, type DragDropEvent} from '@tauri-apps/api/webview';
import {resolveAttachedImage, type AttachedImage} from '@/page/common/util/imageAttach.ts';
import {isPointInsideElement, physicalPositionToCssPoint, resolveWebviewDragDevicePixelRatio} from '@/page/common/util/sftpUtil.ts';

interface DropPasteProps {
    editor: Editor | null;
    remainingSlots: number;
    onDropOverChange: (over: boolean) => void;
    panelRef: RefObject<HTMLDivElement | null>;
    onAddImages: (images: AttachedImage[]) => void;
}

export function DropPaste(props: DropPasteProps) {
    const {editor, remainingSlots, panelRef, onAddImages, onDropOverChange} = props;
    const systemUtilService = useSystemUtilService();

    function insertLocalPaths(paths: string[]): void {
        if (!editor) {
            return;
        }
        for (const path of paths) {
            if (isEmpty(path)) {
                continue;
            }
            const title = resolveLocalPathName(path);
            if (isEmpty(title)) {
                continue;
            }
            const reference: ChatReference = {kind: CHAT_TARGET_TYPE.LOCAL_PATH, id: path, title: title, data: null};
            insertEditorReference(editor, reference, null);
        }
    }

    function extractPasteImageFiles(data: DataTransfer): File[] {
        const result: File[] = [];
        for (const item of Array.from(data.items)) {
            if (item.kind !== 'file') {
                continue;
            }
            if (!item.type.startsWith('image/')) {
                continue;
            }
            const file = item.getAsFile();
            if (!file) {
                continue;
            }
            result.push(file);
        }
        return result;
    }

    function addPasteImages(files: File[]): void {
        if (remainingSlots <= 0) {
            return;
        }
        const selected = files.slice(0, remainingSlots);
        Promise.all(selected.map(resolveAttachedImage)).then((list) => {
            const valid = list.filter(isNotNil);
            if (valid.length === 0) {
                return;
            }
            onAddImages(valid);
        }).catch((err) => {
            console.error('[input] paste image error', err);
        });
    }

    function handleOsPaste(event: ClipboardEvent): void {
        const data = event.clipboardData;
        if (!data) {
            return;
        }
        const imageFiles = extractPasteImageFiles(data);
        if (imageFiles.length > 0) {
            event.preventDefault();
            addPasteImages(imageFiles);
            return;
        }
        if (!data.types.includes('Files')) {
            return;
        }
        event.preventDefault();
        systemUtilService.readClipboardLocalPaths().then((paths) => {
            if (paths.length === 0) {
                return;
            }
            insertLocalPaths(paths);
        }).catch((err) => {
            console.error('[input] read clipboard paths error', err);
        });
    }

    function handleOsDragDrop(payload: DragDropEvent): void {
        if (payload.type === 'leave') {
            onDropOverChange(false);
            return;
        }
        const panel = panelRef.current;
        if (!panel) {
            return;
        }
        const point = physicalPositionToCssPoint(payload.position, resolveWebviewDragDevicePixelRatio());
        const inside = isPointInsideElement(panel, point.x, point.y);
        if (payload.type === 'drop') {
            onDropOverChange(false);
            if (!inside) {
                return;
            }
            insertLocalPaths(payload.paths);
            return;
        }
        onDropOverChange(inside);
    }

    useEffect(() => {
        if (!editor) {
            return;
        }
        const dom = editor.view.dom;
        const handler = (event: ClipboardEvent): void => {
            handleOsPaste(event);
        };
        dom.addEventListener('paste', handler);
        return () => {
            dom.removeEventListener('paste', handler);
        };
    }, [editor]);

    useEffect(() => {
        if (!inTauri()) {
            return;
        }
        let disposed = false;
        let unlisten: (() => void) | undefined;
        getCurrentWebview().onDragDropEvent((event) => {
            handleOsDragDrop(event.payload);
        }).then((off) => {
            if (disposed) {
                off();
                return;
            }
            unlisten = off;
        });
        return () => {
            disposed = true;
            unlisten?.();
        };
    }, [editor]);

    return null;
}
