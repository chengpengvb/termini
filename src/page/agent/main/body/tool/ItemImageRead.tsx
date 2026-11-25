import {Ref} from 'react';
import {useI18n} from '@/common/i18n/useI18n.ts';
import type {ItemHandle} from '@/page/common/dto/agent/ItemHandle.ts';
import type {FileReadToolTitles} from '@/page/common/dto/agent/ToolView.ts';
import {FileReadToolView} from '@/page/agent/main/body/tool/common/FileReadToolView.tsx';

interface ItemImageReadProps {
    sessionId: string;
    ref?: Ref<ItemHandle>;
}

export function ItemImageRead(props: ItemImageReadProps) {
    const {ref, sessionId} = props;
    const {t} = useI18n();

    function resolveTitles(): FileReadToolTitles {
        return {
            reading: t('agent.tool.imageReadRunning'),
            done: t('agent.tool.imageReadDone'),
            detached: t('agent.tool.imageReadDetached'),
            failed: t('agent.tool.imageReadFailed'),
            cancelled: t('agent.tool.imageReadCancelled'),
        };
    }

    return (
        <FileReadToolView ref={ref} markdown titles={resolveTitles()} cancelSessionId={sessionId}/>
    );
}
