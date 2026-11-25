import {Ref} from 'react';
import {useI18n} from '@/common/i18n/useI18n.ts';
import type {ItemHandle} from '@/page/common/dto/agent/ItemHandle.ts';
import type {FileReadToolTitles} from '@/page/common/dto/agent/ToolView.ts';
import {FileReadToolView} from '@/page/agent/main/body/tool/common/FileReadToolView.tsx';

interface ItemCodeMapProps {
    sessionId: string;
    ref?: Ref<ItemHandle>;
}

export function ItemCodeMap(props: ItemCodeMapProps) {
    const {ref} = props;
    const {t} = useI18n();

    function resolveTitles(): FileReadToolTitles {
        return {
            reading: t('agent.tool.codeMapRunning'),
            done: t('agent.tool.codeMapDone'),
        };
    }

    return (
        <FileReadToolView ref={ref} titles={resolveTitles()}/>
    );
}
