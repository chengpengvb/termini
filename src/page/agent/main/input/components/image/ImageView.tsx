import { Close } from '@mui/icons-material';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { isNil } from '@/common/util/StringUtil.ts';
import { Box, ButtonBase, Dialog } from '@mui/material';
import { OverlayPaper } from '@/page/common/util/scopedDialogUtil.ts';

interface ImageViewProps {
  url: string | null;
  onClose: () => void;
}

export function ImageView(props: ImageViewProps) {
  const { t } = useI18n();
  const { url, onClose } = props;

  if (isNil(url)) {
    return null;
  }

  return (
    <Dialog open onClose={onClose} maxWidth={false} slots={{ paper: OverlayPaper }} slotProps={{ paper: { className: 'tm-agent-image-preview-paper' } }}>
      <Box className="tm-agent-image-preview">
        <img src={url} alt="" className="tm-agent-image-preview-img" />
        <ButtonBase disableRipple onClick={onClose} aria-label={t('common.close')} className="tm-agent-image-preview-close">
          <Close sx={{ fontSize: 18 }} />
        </ButtonBase>
      </Box>
    </Dialog>
  );
}
