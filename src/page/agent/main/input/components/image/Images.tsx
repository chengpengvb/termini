import {useState} from 'react';
import {Close} from '@mui/icons-material';
import {Box, ButtonBase} from '@mui/material';
import {useI18n} from '@/common/i18n/useI18n.ts';
import type {MouseEvent as ReactMouseEvent} from 'react';
import type {AttachedImage} from '@/page/common/util/imageAttach.ts';
import {ImageView} from '@/page/agent/main/input/components/image/ImageView.tsx';

interface ImagesProps {
    images: AttachedImage[];
    onRemove: (id: string) => void;
}

export function Images(props: ImagesProps) {
    const {t} = useI18n();
    const {images, onRemove} = props;
    const [previewUrl, setPreviewUrl] = useState<string | null>(null);

    function handleOpenPreview(url: string): void {
        setPreviewUrl(url);
    }

    function handleClosePreview(): void {
        setPreviewUrl(null);
    }

    function handleRemove(e: ReactMouseEvent<HTMLElement>, id: string): void {
        e.stopPropagation();
        onRemove(id);
    }

    if (images.length === 0) {
        return null;
    }

    return (
        <Box className="tm-agent-input-attachments">
            {images.map(item => (
                <ButtonBase disableRipple key={item.id} onClick={() => handleOpenPreview(item.url)} className="tm-agent-input-attachment">
                    <img src={item.url} alt="" className="tm-agent-input-attachment-img"/>
                    <Box component="span" onClick={(e) => handleRemove(e, item.id)} aria-label={t('agent.mount.removeImage')} className="tm-agent-input-attachment-remove">
                        <Close sx={{fontSize: 12}}/>
                    </Box>
                </ButtonBase>
            ))}
            <ImageView url={previewUrl} onClose={handleClosePreview}/>
        </Box>
    );
}
