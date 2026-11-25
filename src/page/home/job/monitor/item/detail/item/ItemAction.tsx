import {useState} from 'react';
import {Box, Button} from '@mui/material';
import {useI18n} from '@/common/i18n/useI18n.ts';
import {isEmpty} from '@/common/util/StringUtil.ts';
import {isDraftLink} from '@/page/common/util/jobUtil.ts';
import type {JobIndicator} from '@/common/model/JobIndicator.ts';
import {resolveErrorText} from '@/common/model/resp/ResultModel.ts';
import type {JobDevice2Indicator} from '@/common/model/JobDevice.ts';
import {useJobIndicatorService} from '@/common/context/useAppHooks.tsx';

export interface ItemActionProps {
    draft: JobIndicator;
    jobDeviceId: string;
    intervalSecs: number;
    onDeleted: () => void;
    link: JobDevice2Indicator;
    onError: (message: string | null) => void;
    onSaved: (link: JobDevice2Indicator) => void;
}

export function ItemAction(props: ItemActionProps) {
    const {t} = useI18n();
    const [saving, setSaving] = useState(false);
    const [deleting, setDeleting] = useState(false);
    const jobIndicatorService = useJobIndicatorService();
    const {link, draft, jobDeviceId, intervalSecs, onSaved, onDeleted, onError} = props;
    
    const handleSave = (): void => {
        if (saving || deleting || isEmpty(draft.name)) {
            return;
        }
        setSaving(true);
        onError(null);
        void jobIndicatorService.saveWithJob({interval_secs: intervalSecs, job_device_id: jobDeviceId, job_indicator: draft}).then(saved => {
            setSaving(false);
            onSaved(saved);
        }).catch(err => {
            setSaving(false);
            onError(resolveErrorText(err));
        });
    };

    const handleDelete = (): void => {
        if (saving || deleting) {
            return;
        }
        if (isDraftLink(link)) {
            onDeleted();
            return;
        }
        setDeleting(true);
        onError(null);
        void jobIndicatorService.deleteWithJob(link.id).then(() => {
            onDeleted();
        }).catch(err => {
            setDeleting(false);
            onError(resolveErrorText(err));
        });
    };

    return (
        <Box className="tm-job-drawer-actions">
            <Button disableRipple variant="contained" className="tm-btn tm-btn-danger" disabled={saving || deleting} onClick={handleDelete}>
                {deleting ? t('home.job.deleting') : t('common.delete')}
            </Button>
            <Button disableRipple variant="contained" className="tm-btn tm-btn-primary" disabled={saving || deleting || isEmpty(draft.name)} onClick={handleSave}>
                {saving ? t('home.job.saving') : t('common.save')}
            </Button>
        </Box>
    );
}
