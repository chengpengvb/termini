import {Ref} from 'react';
import {useI18n} from '@/common/i18n/useI18n.ts';
import type {ItemHandle} from '@/page/common/dto/agent/ItemHandle.ts';
import type {FileReadToolTitles} from '@/page/common/dto/agent/ToolView.ts';
import {FileReadToolView} from '@/page/agent/main/body/tool/common/FileReadToolView.tsx';

interface ItemSftpFileReadProps {
    sessionId: string;
    ref?: Ref<ItemHandle>;
}

export function ItemSftpFileRead(props: ItemSftpFileReadProps) {
    const {ref} = props;
    const {t} = useI18n();

    function resolveTitles(): FileReadToolTitles {
        return {
            reading: t('agent.tool.sftpFileReadRunning'),
            done: t('agent.tool.sftpFileReadDone'),
        };
    }

    return (
        <FileReadToolView ref={ref} titles={resolveTitles()}/>
    );
}
