import { useI18n } from '@/common/i18n/useI18n.ts';
import { useEffect, useRef, useState } from 'react';
import type { AiSkill } from '@/common/model/AiSkill.ts';
import { isNil, isNotNil } from '@/common/util/StringUtil.ts';
import type { RespTree } from '@/common/model/resp/RespTree.ts';
import { CHAT_TARGET_TYPE } from '@/common/const/ChatTargetConst.ts';
import { usePlatformMobile } from '@/common/context/usePlatformMobile';
import type { ChatReference } from '@/common/model/dto/ChatReference.ts';
import { OverlayPopoverPaper } from '@/page/common/util/scopedDialogUtil.ts';
import { ActionSheet } from '@/page/common/components/dialog/ActionSheet.tsx';
import { SkillPick } from '@/page/agent/main/input/components/tool/SkillPick.tsx';
import { TargetPick } from '@/page/agent/main/input/components/tool/TargetPick.tsx';
import { useAiSkillService, useDeviceService } from '@/common/context/useAppHooks.tsx';
import { Box, ButtonBase, Popover, Typography, type PopoverActions } from '@mui/material';
import { resolveAttachedImage, type AttachedImage } from '@/page/common/util/imageAttach.ts';
import { Add, ArrowBack, BuildOutlined, DnsOutlined, ImageOutlined, LayersOutlined } from '@mui/icons-material';

interface AddMenuProps {
  remainingSlots: number;
  onAddImages: (images: AttachedImage[]) => void;
  onInsertReference: (ref: ChatReference) => void;
}

export function AddMenu(props: AddMenuProps) {
  const { t } = useI18n();
  const aiSkillService = useAiSkillService();
  const deviceService = useDeviceService();
  const mobileViewport = usePlatformMobile();
  const [open, setOpen] = useState<boolean>(false);
  const [skills, setSkills] = useState<AiSkill[]>([]);
  const [tree, setTree] = useState<RespTree[]>([]);
  const triggerRef = useRef<HTMLButtonElement | null>(null);
  const fileInputRef = useRef<HTMLInputElement | null>(null);
  const mountBodyRef = useRef<HTMLDivElement | null>(null);
  const popoverActionRef = useRef<PopoverActions | null>(null);
  const { remainingSlots, onAddImages, onInsertReference } = props;
  const [pickerKind, setPickerKind] = useState<number | null>(null);

  function handleOpen(): void {
    setPickerKind(null);
    setOpen(true);
  }

  function handleClose(): void {
    setOpen(false);
  }

  function handleExited(): void {
    setPickerKind(null);
  }

  function handleBack(): void {
    setPickerKind(null);
  }

  function handleOpenSkillPicker(): void {
    aiSkillService
      .selectAll()
      .then(list => {
        setSkills([...list].sort((a, b) => a.sort - b.sort));
        setPickerKind(CHAT_TARGET_TYPE.SKILL);
      })
      .catch(err => {
        console.error('[input] load skills error', err);
      });
  }

  function handleOpenTargetPicker(kind: number): void {
    deviceService
      .selectDeviceTree()
      .then(next => {
        setTree(next);
        setPickerKind(kind);
      })
      .catch(err => {
        console.error('[input] load reference tree error', err);
      });
  }

  function handlePickImage(): void {
    handleClose();
    const input = fileInputRef.current;
    if (isNil(input)) {
      return;
    }
    input.click();
  }

  function handlePickReference(ref: ChatReference): void {
    handleClose();
    onInsertReference(ref);
  }

  function handleFilesSelected(fileList: FileList | null): void {
    if (isNil(fileList)) {
      return;
    }
    if (remainingSlots <= 0) {
      return;
    }
    const files = Array.from(fileList).slice(0, remainingSlots);
    Promise.all(files.map(resolveAttachedImage))
      .then(parsedList => {
        const valid = parsedList.filter(isNotNil);
        if (valid.length === 0) {
          return;
        }
        onAddImages(valid);
      })
      .catch(err => {
        console.error('[input] read image error', err);
      });
  }

  function resolvePickerTitle(): string {
    if (pickerKind === CHAT_TARGET_TYPE.GROUP) {
      return t('agent.mount.pickGroup');
    }
    if (pickerKind === CHAT_TARGET_TYPE.SKILL) {
      return t('agent.mount.pickSkill');
    }
    return t('agent.mount.pickDevice');
  }

  useEffect(() => {
    if (!open) {
      return;
    }
    if (mobileViewport) {
      return;
    }
    const el = mountBodyRef.current;
    if (isNil(el)) {
      return;
    }
    const ro = new ResizeObserver(() => {
      popoverActionRef.current?.updatePosition();
    });
    ro.observe(el);
    popoverActionRef.current?.updatePosition();
    return () => {
      ro.disconnect();
    };
  }, [open, pickerKind, mobileViewport, skills, tree]);

  const menuBody = isNotNil(pickerKind) ? (
    <Box ref={mountBodyRef} className={mobileViewport ? 'tm-agent-mount is-sheet' : 'tm-agent-mount'}>
      <ButtonBase disableRipple onClick={handleBack} className="tm-agent-mount-back">
        <ArrowBack sx={{ fontSize: 14 }} />
        <Typography className="tm-agent-mount-back-text">{resolvePickerTitle()}</Typography>
      </ButtonBase>
      {pickerKind === CHAT_TARGET_TYPE.SKILL ? <SkillPick skills={skills} onPick={handlePickReference} /> : <TargetPick kind={pickerKind} tree={tree} onPick={handlePickReference} />}
    </Box>
  ) : (
    <Box ref={mountBodyRef} className={mobileViewport ? 'tm-agent-mount is-sheet' : 'tm-agent-mount'}>
      <Typography className="tm-agent-mount-group-label">{t('agent.mount.add')}</Typography>
      <ButtonBase disableRipple disabled={remainingSlots <= 0} onClick={handlePickImage} className="tm-agent-mount-option">
        <Box className="tm-agent-mount-option-icon">
          <ImageOutlined sx={{ fontSize: 16 }} />
        </Box>
        <Box className="tm-agent-mount-option-text">
          <Typography className="tm-agent-mount-option-name">{t('agent.mount.image')}</Typography>
          <Typography className="tm-agent-mount-option-desc">{t('agent.mount.imageDesc')}</Typography>
        </Box>
      </ButtonBase>
      <ButtonBase disableRipple onClick={handleOpenSkillPicker} className="tm-agent-mount-option">
        <Box className="tm-agent-mount-option-icon">
          <BuildOutlined sx={{ fontSize: 16 }} />
        </Box>
        <Box className="tm-agent-mount-option-text">
          <Typography className="tm-agent-mount-option-name">{t('agent.mount.skill')}</Typography>
          <Typography className="tm-agent-mount-option-desc">{t('agent.mount.skillDesc')}</Typography>
        </Box>
      </ButtonBase>
      <ButtonBase disableRipple onClick={() => handleOpenTargetPicker(CHAT_TARGET_TYPE.DEVICE)} className="tm-agent-mount-option">
        <Box className="tm-agent-mount-option-icon">
          <DnsOutlined sx={{ fontSize: 16 }} />
        </Box>
        <Box className="tm-agent-mount-option-text">
          <Typography className="tm-agent-mount-option-name">{t('agent.mount.device')}</Typography>
          <Typography className="tm-agent-mount-option-desc">{t('agent.mount.deviceDesc')}</Typography>
        </Box>
      </ButtonBase>
      <ButtonBase disableRipple onClick={() => handleOpenTargetPicker(CHAT_TARGET_TYPE.GROUP)} className="tm-agent-mount-option">
        <Box className="tm-agent-mount-option-icon">
          <LayersOutlined sx={{ fontSize: 16 }} />
        </Box>
        <Box className="tm-agent-mount-option-text">
          <Typography className="tm-agent-mount-option-name">{t('agent.mount.group')}</Typography>
          <Typography className="tm-agent-mount-option-desc">{t('agent.mount.groupDesc')}</Typography>
        </Box>
      </ButtonBase>
    </Box>
  );

  return (
    <>
      <ButtonBase ref={triggerRef} disableRipple onClick={handleOpen} aria-label={t('agent.mount.addAria')} className="tm-agent-input-add">
        <Add sx={{ fontSize: 16 }} />
      </ButtonBase>
      <input
        ref={fileInputRef}
        type="file"
        accept="image/*"
        multiple
        hidden
        onChange={e => {
          handleFilesSelected(e.target.files);
          e.target.value = '';
        }}
      />
      {mobileViewport ? (
        <ActionSheet open={open} onClose={handleClose}>
          {menuBody}
        </ActionSheet>
      ) : (
        <Popover
          action={popoverActionRef}
          open={open}
          anchorEl={triggerRef.current}
          onClose={handleClose}
          disableRestoreFocus
          disableAutoFocus
          marginThreshold={8}
          transitionDuration={{ enter: 120, exit: 0 }}
          anchorOrigin={{ vertical: 'top', horizontal: 'left' }}
          transformOrigin={{ vertical: 'bottom', horizontal: 'left' }}
          slots={{ paper: OverlayPopoverPaper }}
          slotProps={{ paper: { className: 'tm-agent-mount-paper' }, transition: { onExited: handleExited } }}
        >
          {menuBody}
        </Popover>
      )}
    </>
  );
}
