import {createElement} from 'react';
import {FileEditPage} from '@/page/fileedit/FileEditPage.tsx';
import type {TabManagerService} from '@/common/components/tab';
import {isNotEmpty, isNotNil} from '@/common/util/StringUtil.ts';
import type {SystemUtilService} from '@/service/SystemUtilService.ts';
import {isTextEditableFile, resolveFileName, resolveFileTypeIcon} from '@/page/common/util/fileTypeUtil.ts';

const FILE_EDIT_PANE_TYPE = 1;

function findOpenedTabId(tabManagerService: TabManagerService, path: string): string | null {
    for (const tab of tabManagerService.getTabs()) {
        for (const pane of tab.panes) {
            if (pane.data?.type !== FILE_EDIT_PANE_TYPE) {
                continue;
            }
            if (pane.data.data?.path === path && isNotEmpty(tab.id)) {
                return tab.id;
            }
        }
    }
    return null;
}

export function openFileEditTab(tabManagerService: TabManagerService, systemUtilService: SystemUtilService, path: string): void {
    if (!isTextEditableFile(path)) {
        systemUtilService.revealLocalPath(path).catch(error => {
            console.error('[fileEdit] reveal local path failed', error);
        });
        return;
    }
    const openedTabId = findOpenedTabId(tabManagerService, path);
    if (isNotNil(openedTabId)) {
        tabManagerService.setActiveTabId(openedTabId);
        return;
    }
    tabManagerService.openTab({
        panes: [{
            tabBar: {
                canClose: true,
                title: resolveFileName(path),
                icon: resolveFileTypeIcon(path, 'tm-tab-file-icon'),
            },
            data: {type: FILE_EDIT_PANE_TYPE, data: {path}},
            content: createElement(FileEditPage, {path}),
        }],
    });
}
