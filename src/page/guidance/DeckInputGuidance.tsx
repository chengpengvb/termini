import { Box } from '@mui/material';
import { createPortal } from 'react-dom';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { useEffect, useRef, useState } from 'react';
import { isRdpKeyboardTarget } from '@/page/common/util/rdpUtil.ts';
import { usePlatformMobile } from '@/common/context/usePlatformMobile';
import { useAccountService, useTabManagerService } from '@/common/context/useAppHooks.tsx';
import {
  GUIDANCE_BEAK_BOTTOM,
  GUIDANCE_BEAK_H,
  GUIDANCE_BEAK_TOP,
  GUIDANCE_BEAK_W,
  GUIDANCE_DECK_INPUT_STEPS,
  GUIDANCE_SPOT_PAD,
  GUIDANCE_TAB_DECK,
  GUIDANCE_TIP_WIDTH,
  guidanceHasClosableTab,
  guidanceQueryDeckInputEditor,
  guidanceQueryDeckInputPanel,
  guidanceReadTarget,
  type GuidanceTarget,
} from '@/page/common/util/guidanceSurface.ts';

export function DeckInputGuidance() {
  const { t } = useI18n();
  const mobile = usePlatformMobile();
  const [armed, setArmed] = useState(false);
  const accountService = useAccountService();
  const tipRef = useRef<HTMLDivElement>(null);
  const spotRef = useRef<HTMLDivElement>(null);
  const beakRef = useRef<HTMLDivElement>(null);
  const [stepIndex, setStepIndex] = useState(0);
  const tabManagerService = useTabManagerService();
  const [beakSide, setBeakSide] = useState<number>(GUIDANCE_BEAK_TOP);
  const [done, setDone] = useState(() => accountService.selectDeckInputOnboardDone());
  const chromeOpen = !accountService.selectOnboardDone() && !guidanceHasClosableTab(tabManagerService);
  const visible = armed && !done && !chromeOpen && !accountService.selectDeckInputOnboardDone();

  function currentStep(): (typeof GUIDANCE_DECK_INPUT_STEPS)[number] {
    const next = GUIDANCE_DECK_INPUT_STEPS[stepIndex];
    if (next === undefined) {
      throw new Error(`deck input guidance step missing: ${stepIndex}`);
    }
    return next;
  }

  function finish(): void {
    accountService.saveDeckInputOnboardDone();
    setDone(true);
    setArmed(false);
  }

  function handleSkip(): void {
    finish();
  }

  function handleNext(): void {
    if (stepIndex >= GUIDANCE_DECK_INPUT_STEPS.length - 1) {
      finish();
      return;
    }
    setStepIndex(current => current + 1);
  }

  function handleBack(): void {
    setStepIndex(current => {
      if (current === 0) {
        return current;
      }
      return current - 1;
    });
  }

  function hideOverlay(): void {
    const spot = spotRef.current;
    const tip = tipRef.current;
    if (spot !== null) {
      spot.style.visibility = 'hidden';
    }
    if (tip !== null) {
      tip.style.visibility = 'hidden';
    }
  }

  function writeSpot(nextTarget: GuidanceTarget): void {
    const spot = spotRef.current;
    if (spot === null) {
      throw new Error('deck input guidance spot missing');
    }
    spot.style.visibility = 'visible';
    spot.style.left = `${nextTarget.left - GUIDANCE_SPOT_PAD}px`;
    spot.style.top = `${nextTarget.top - GUIDANCE_SPOT_PAD}px`;
    spot.style.width = `${nextTarget.width + GUIDANCE_SPOT_PAD * 2}px`;
    spot.style.height = `${nextTarget.height + GUIDANCE_SPOT_PAD * 2}px`;
    spot.style.borderRadius = nextTarget.radius;
  }

  function writeTip(spotTarget: GuidanceTarget, tipTarget: GuidanceTarget): void {
    const tip = tipRef.current;
    const beak = beakRef.current;
    if (tip === null) {
      throw new Error('deck input guidance tip missing');
    }
    if (beak === null) {
      throw new Error('deck input guidance beak missing');
    }
    const vw = window.innerWidth;
    const vh = window.innerHeight;
    const tipH = tip.offsetHeight;
    const tipW = Math.min(GUIDANCE_TIP_WIDTH, vw - 24);
    const spaceBelow = vh - tipTarget.top - tipTarget.height;
    const placeAbove = mobile || spaceBelow < tipH + 16;
    const side = placeAbove ? GUIDANCE_BEAK_BOTTOM : GUIDANCE_BEAK_TOP;
    let left = tipTarget.left;
    let top = placeAbove ? tipTarget.top - tipH - 8 : tipTarget.top + tipTarget.height + 8;
    let at = 22;
    if (left < 12) {
      left = 12;
    }
    if (left + tipW > vw - 12) {
      left = vw - tipW - 12;
    }
    if (top + tipH > vh - 12) {
      top = vh - tipH - 12;
    }
    if (top < 8) {
      top = 8;
    }
    const spotCenter = spotTarget.left + spotTarget.width / 2 - left;
    at = spotCenter;
    if (at < 22) {
      at = 22;
    }
    if (at > tipW - 22) {
      at = tipW - 22;
    }
    tip.style.visibility = 'visible';
    tip.style.left = `${left}px`;
    tip.style.top = `${top}px`;
    setBeakSide(side);
    beak.style.left = `${at - GUIDANCE_BEAK_W / 2}px`;
    if (side === GUIDANCE_BEAK_BOTTOM) {
      beak.style.top = `${tipH - 1}px`;
    } else {
      beak.style.top = `${-GUIDANCE_BEAK_H + 1}px`;
    }
  }

  useEffect(() => {
    const onClick = (event: MouseEvent): void => {
      const node = event.target;
      if (!(node instanceof Element)) {
        return;
      }
      const tab = node.closest(`[data-tab-id="${GUIDANCE_TAB_DECK}"]`);
      if (!(tab instanceof HTMLElement)) {
        return;
      }
      if (accountService.selectDeckInputOnboardDone()) {
        return;
      }
      if (!accountService.selectOnboardDone() && !guidanceHasClosableTab(tabManagerService)) {
        return;
      }
      setStepIndex(0);
      setArmed(true);
    };
    document.addEventListener('click', onClick, true);
    return () => {
      document.removeEventListener('click', onClick, true);
    };
  }, [accountService, tabManagerService]);

  useEffect(() => {
    if (!visible) {
      return;
    }
    hideOverlay();
    let hadTarget = false;
    let frame = 0;
    const tick = (): void => {
      const editor = guidanceQueryDeckInputEditor();
      const panel = guidanceQueryDeckInputPanel();
      const spotTarget = editor === null ? null : guidanceReadTarget(editor);
      const tipTarget = panel === null ? null : guidanceReadTarget(panel);
      if (spotTarget === null || tipTarget === null) {
        if (hadTarget) {
          throw new Error('deck input guidance anchor missing');
        }
        frame = window.requestAnimationFrame(tick);
        return;
      }
      hadTarget = true;
      writeSpot(spotTarget);
      writeTip(spotTarget, tipTarget);
      frame = window.requestAnimationFrame(tick);
    };
    frame = window.requestAnimationFrame(tick);
    return () => {
      window.cancelAnimationFrame(frame);
    };
  }, [visible, stepIndex, mobile]);

  useEffect(() => {
    if (!visible) {
      return;
    }
    const onKey = (event: KeyboardEvent): void => {
      if (isRdpKeyboardTarget(event.target)) {
        return;
      }
      if (event.key === 'Escape') {
        finish();
      }
      if (event.key === 'ArrowRight') {
        handleNext();
      }
      if (event.key === 'ArrowLeft') {
        handleBack();
      }
    };
    window.addEventListener('keydown', onKey);
    return () => {
      window.removeEventListener('keydown', onKey);
    };
  }, [visible, stepIndex]);

  if (!visible) {
    return null;
  }

  const step = currentStep();
  const last = stepIndex === GUIDANCE_DECK_INPUT_STEPS.length - 1;
  const first = stepIndex === 0;

  return createPortal(
    <Box className="tm-guidance-root">
      <Box ref={spotRef} className="tm-guidance-spot" style={{ visibility: 'hidden' }} />
      <Box ref={tipRef} className="tm-guidance-tip" style={{ visibility: 'hidden' }}>
        <Box ref={beakRef} className={beakSide === GUIDANCE_BEAK_BOTTOM ? 'tm-guidance-beak is-bottom' : 'tm-guidance-beak'}>
          {beakSide === GUIDANCE_BEAK_BOTTOM ? (
            <svg width="14" height="7" viewBox="0 0 14 7">
              <path d="M0 0 L7 6.5 L14 0" fill="var(--tm-glass-dialog-bg)" />
              <path d="M0.5 0.2 L7 6.2 L13.5 0.2" fill="none" stroke="var(--tm-glass-dialog-border)" strokeWidth="1" strokeLinejoin="round" />
            </svg>
          ) : (
            <svg width="14" height="7" viewBox="0 0 14 7">
              <path d="M0 7 L7 0.5 L14 7" fill="var(--tm-glass-dialog-bg)" />
              <path d="M0.5 6.8 L7 0.8 L13.5 6.8" fill="none" stroke="var(--tm-glass-dialog-border)" strokeWidth="1" strokeLinejoin="round" />
            </svg>
          )}
        </Box>
        <Box className="tm-guidance-title">{t(step.titleKey)}</Box>
        <Box className="tm-guidance-body">{t(step.bodyKey)}</Box>
        <Box className="tm-guidance-foot">
          <button type="button" className="tm-guidance-skip" onClick={handleSkip}>
            {t('guidance.skip')}
          </button>
          <Box className="tm-guidance-nav">
            <button type="button" className={first ? 'tm-guidance-btn is-off' : 'tm-guidance-btn'} disabled={first} onClick={handleBack}>
              {t('guidance.back')}
            </button>
            <button type="button" className="tm-guidance-btn" onClick={handleNext}>
              {last ? t('guidance.finish') : t('guidance.next')}
            </button>
          </Box>
        </Box>
      </Box>
    </Box>,
    document.body,
  );
}
