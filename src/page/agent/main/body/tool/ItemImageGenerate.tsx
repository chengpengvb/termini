import {Ref} from 'react';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {FilePreview} from '@/page/agent/common/FilePreview.tsx';
import type {ItemHandle} from '@/page/common/dto/agent/ItemHandle.ts';
import type {FileReadToolTitles} from '@/page/common/dto/agent/ToolView.ts';
import {FileReadToolView} from '@/page/agent/main/body/tool/common/FileReadToolView.tsx';

interface ItemImageGenerateProps {
    sessionId: string;
    ref?: Ref<ItemHandle>;
}

export function ItemImageGenerate(props: ItemImageGenerateProps) {
    const {ref, sessionId} = props;
    const {t} = useI18n();

    function resolveTitles(): FileReadToolTitles {
        return {
            reading: t('agent.tool.imageGenerateRunning'),
            done: t('agent.tool.imageGenerateDone'),
            detached: t('agent.tool.imageGenerateDetached'),
            failed: t('agent.tool.imageGenerateFailed'),
            cancelled: t('agent.tool.imageGenerateCancelled'),
        };
    }

    return (
        <FileReadToolView ref={ref} markdown titles={resolveTitles()} cancelSessionId={sessionId} pending={<FilePreview generating/>}/>
    );
}
