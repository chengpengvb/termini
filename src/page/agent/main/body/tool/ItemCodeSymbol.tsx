import {Ref} from 'react';
import {useI18n} from '@/common/i18n/useI18n.ts';
import type {ItemHandle} from '@/page/common/dto/agent/ItemHandle.ts';
import type {FileReadToolTitles} from '@/page/common/dto/agent/ToolView.ts';
import {FileReadToolView} from '@/page/agent/main/body/tool/common/FileReadToolView.tsx';

interface ItemCodeSymbolProps {
    sessionId: string;
    ref?: Ref<ItemHandle>;
}

export function ItemCodeSymbol(props: ItemCodeSymbolProps) {
    const {ref} = props;
    const {t} = useI18n();

    function resolveTitles(): FileReadToolTitles {
        return {
            reading: t('agent.tool.codeSymbolRunning'),
            done: t('agent.tool.codeSymbolDone'),
        };
    }

    return (
        <FileReadToolView ref={ref} titles={resolveTitles()}/>
    );
}
