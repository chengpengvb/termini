import { Box } from '@mui/material';
import { createPortal } from 'react-dom';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { useEffect, useRef, useState } from 'react';
import { EVENT_TYPE } from '@/common/components/tab/core/types.ts';
import { isRdpKeyboardTarget } from '@/page/common/util/rdpUtil.ts';
import { useAccountService, useTabManagerService } from '@/common/context/useAppHooks.tsx';
import {
  GUIDANCE_BEAK_H,
  GUIDANCE_BEAK_LEFT,
  GUIDANCE_BEAK_TOP,
  GUIDANCE_BEAK_W,
  GUIDANCE_SPOT_PAD,
  GUIDANCE_STEPS,
  GUIDANCE_TIP_WIDTH,
  guidanceApplyMenu,
  guidanceApplyTab,
  guidanceHasClosableTab,
  guidanceHasHomeMenu,
  guidanceHasTabs,
  guidanceIsTabStep,
  guidanceNeedsHomeMenu,
  guidanceQueryAnchor,
  guidanceReadTarget,
  type GuidanceTarget,
} from '@/page/common/util/guidanceSurface.ts';

export function GuidanceRoot() {
  const { t } = useI18n();
  const [done, setDone] = useState(false);
  const accountService = useAccountService();
  const tipRef = useRef<HTMLDivElement>(null);
  const spotRef = useRef<HTMLDivElement>(null);
  const beakRef = useRef<HTMLDivElement>(null);
  const [stepIndex, setStepIndex] = useState(0);
  const tabManagerService = useTabManagerService();
  const [beakSide, setBeakSide] = useState<number>(GUIDANCE_BEAK_TOP);
  const [used, setUsed] = useState(() => guidanceHasClosableTab(tabManagerService));
  const visible = !done && !used && !accountService.selectOnboardDone();

  function currentStep(): (typeof GUIDANCE_STEPS)[number] {
    const next = GUIDANCE_STEPS[stepIndex];
    if (next === undefined) {
      throw new Error(`guidance step missing: ${stepIndex}`);
    }
    return next;
  }

  function finish(): void {
    accountService.saveOnboardDone();
    setDone(true);
  }

  function handleSkip(): void {
    finish();
  }

  function handleNext(): void {
    if (stepIndex >= GUIDANCE_STEPS.length - 1) {
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
      throw new Error('guidance spot missing');
    }
    spot.style.visibility = 'visible';
    spot.style.left = `${nextTarget.left - GUIDANCE_SPOT_PAD}px`;
    spot.style.top = `${nextTarget.top - GUIDANCE_SPOT_PAD}px`;
    spot.style.width = `${nextTarget.width + GUIDANCE_SPOT_PAD * 2}px`;
    spot.style.height = `${nextTarget.height + GUIDANCE_SPOT_PAD * 2}px`;
    spot.style.borderRadius = nextTarget.radius;
  }

  function writeTip(nextTarget: GuidanceTarget, id: string): void {
    const tip = tipRef.current;
    const beak = beakRef.current;
    if (tip === null) {
      throw new Error('guidance tip missing');
    }
    if (beak === null) {
      throw new Error('guidance beak missing');
    }
    const vw = window.innerWidth;
    const vh = window.innerHeight;
    const tipW = Math.min(GUIDANCE_TIP_WIDTH, vw - 24);
    const tipH = tip.offsetHeight;
    const side = vw < 600 || guidanceIsTabStep(id) ? GUIDANCE_BEAK_TOP : GUIDANCE_BEAK_LEFT;
    let left: number;
    let top: number;
    let at: number;
    if (side === GUIDANCE_BEAK_TOP) {
      left = id === 'avatar' ? nextTarget.left + nextTarget.width - tipW : nextTarget.left;
      top = nextTarget.top + nextTarget.height + 8;
      at = id === 'avatar' ? tipW - 28 : 22;
    } else {
      left = nextTarget.left + nextTarget.width + GUIDANCE_SPOT_PAD + 10;
      top = nextTarget.top + nextTarget.height / 2 - tipH / 2;
      at = nextTarget.top + nextTarget.height / 2 - top;
    }
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
    if (side === GUIDANCE_BEAK_LEFT) {
      at = nextTarget.top + nextTarget.height / 2 - top;
    }
    tip.style.visibility = 'visible';
    tip.style.left = `${left}px`;
    tip.style.top = `${top}px`;
    setBeakSide(side);
    if (side === GUIDANCE_BEAK_TOP) {
      beak.style.left = `${at - GUIDANCE_BEAK_W / 2}px`;
      beak.style.top = `${-GUIDANCE_BEAK_H + 1}px`;
    } else {
      let clamp = nextTarget.top + nextTarget.height / 2 - top - GUIDANCE_BEAK_H;
      if (clamp < 12) {
        clamp = 12;
      }
      if (clamp > tipH - 26) {
        clamp = tipH - 26;
      }
      beak.style.top = `${clamp}px`;
      beak.style.left = `${-GUIDANCE_BEAK_H + 1}px`;
    }
  }

  useEffect(() => {
    const sync = (): void => {
      if (!guidanceHasClosableTab(tabManagerService)) {
        return;
      }
      setUsed(true);
      if (!accountService.selectOnboardDone()) {
        accountService.saveOnboardDone();
      }
      setDone(true);
    };
    sync();
    return tabManagerService.addEventListener((event: number) => {
      if (event === EVENT_TYPE.OPEN || event === EVENT_TYPE.LAYOUT) {
        sync();
      }
    });
  }, [accountService, tabManagerService]);

  useEffect(() => {
    if (!visible) {
      return;
    }
    const id = currentStep().id;
    hideOverlay();
    let tabApplied = false;
    let menuApplied = !guidanceNeedsHomeMenu(id);
    let hadTarget = false;
    let frame = 0;
    const tick = (): void => {
      if (!guidanceHasTabs()) {
        frame = window.requestAnimationFrame(tick);
        return;
      }
      if (!tabApplied) {
        guidanceApplyTab(id);
        tabApplied = true;
      }
      if (guidanceNeedsHomeMenu(id) && !guidanceHasHomeMenu()) {
        frame = window.requestAnimationFrame(tick);
        return;
      }
      if (!menuApplied) {
        guidanceApplyMenu(id);
        menuApplied = true;
      }
      const el = guidanceQueryAnchor(id);
      const nextTarget = el === null ? null : guidanceReadTarget(el);
      if (nextTarget === null) {
        if (hadTarget) {
          throw new Error(`guidance anchor missing: ${id}`);
        }
        frame = window.requestAnimationFrame(tick);
        return;
      }
      hadTarget = true;
      writeSpot(nextTarget);
      writeTip(nextTarget, id);
      frame = window.requestAnimationFrame(tick);
    };
    frame = window.requestAnimationFrame(tick);
    return () => {
      window.cancelAnimationFrame(frame);
    };
  }, [visible, stepIndex]);

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
  const last = stepIndex === GUIDANCE_STEPS.length - 1;
  const first = stepIndex === 0;

  return createPortal(
    <Box className="tm-guidance-root">
      <Box ref={spotRef} className="tm-guidance-spot" style={{ visibility: 'hidden' }} />
      <Box ref={tipRef} className="tm-guidance-tip" style={{ visibility: 'hidden' }}>
        <Box ref={beakRef} className={beakSide === GUIDANCE_BEAK_LEFT ? 'tm-guidance-beak is-left' : 'tm-guidance-beak'}>
          {beakSide === GUIDANCE_BEAK_TOP ? (
            <svg width="14" height="7" viewBox="0 0 14 7">
              <path d="M0 7 L7 0.5 L14 7" fill="var(--tm-glass-dialog-bg)" />
              <path d="M0.5 6.8 L7 0.8 L13.5 6.8" fill="none" stroke="var(--tm-glass-dialog-border)" strokeWidth="1" strokeLinejoin="round" />
            </svg>
          ) : (
            <svg width="7" height="14" viewBox="0 0 7 14">
              <path d="M7 0 L0.5 7 L7 14" fill="var(--tm-glass-dialog-bg)" />
              <path d="M6.8 0.5 L0.8 7 L6.8 13.5" fill="none" stroke="var(--tm-glass-dialog-border)" strokeWidth="1" strokeLinejoin="round" />
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
