import {Ref} from 'react';
import {useI18n} from '@/common/i18n/useI18n.ts';
import type {ItemHandle} from '@/page/common/dto/agent/ItemHandle.ts';
import type {FileReadToolTitles} from '@/page/common/dto/agent/ToolView.ts';
import {FileReadToolView} from '@/page/agent/main/body/tool/common/FileReadToolView.tsx';

interface ItemCodeSearchProps {
    sessionId: string;
    ref?: Ref<ItemHandle>;
}

export function ItemCodeSearch(props: ItemCodeSearchProps) {
    const {ref} = props;
    const {t, tf} = useI18n();

    function resolveTitles(): FileReadToolTitles {
        return {
            reading: t('agent.tool.codeSearchRunning'),
            done: t('agent.tool.codeSearchDone'),
            batchReading: (count: number) => tf('agent.tool.codeSearchBatchRunning', {count}),
            batchDone: (count: number) => tf('agent.tool.codeSearchBatchDone', {count}),
        };
    }

    return (
        <FileReadToolView ref={ref} titles={resolveTitles()}/>
    );
}
