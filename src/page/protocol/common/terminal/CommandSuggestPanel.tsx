import { useEffect, useRef } from 'react';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { ArrowUpwardRounded, CloseRounded } from '@mui/icons-material';
import type { SuggestAnchor } from '@/page/common/util/commandSuggestUtil.ts';
import type { ReactNode, KeyboardEvent, MouseEvent, PointerEvent } from 'react';

const SKELETON_ROWS = [0, 1, 2];

export interface CommandSuggestPanelProps {
  mobile: boolean;
  width: number;
  typed: string;
  error: string;
  list: string[];
  maxWidth: number;
  loading: boolean;
  selected: number;
  anchor: SuggestAnchor;
  instruction: string;
  onSubmit: () => void;
  onAccept: () => void;
  onClose: () => void;
  onMove: (delta: number) => void;
  onPick: (index: number) => void;
  onHover: (index: number) => void;
  onInstructionChange: (value: string) => void;
}

export function CommandSuggestPanel(props: CommandSuggestPanelProps) {
  const { t } = useI18n();
  const inputRef = useRef<HTMLInputElement | null>(null);
  const textRefs = useRef<Array<HTMLDivElement | null>>([]);
  const { mobile, anchor, width, maxWidth, list, selected, loading, typed, error, instruction } = props;
  const { onInstructionChange, onSubmit, onAccept, onMove, onClose, onPick, onHover } = props;

  const resolveItemClass = (index: number, item: string): string => {
    if (item.includes('\n')) {
      if (index === selected) {
        return 'tm-suggest-item is-selected is-block';
      }
      return 'tm-suggest-item is-block';
    }
    if (index === selected) {
      return 'tm-suggest-item is-selected';
    }
    return 'tm-suggest-item';
  };

  const handleItemEnter = (event: MouseEvent<HTMLDivElement>, index: number, full: string): void => {
    onHover(index);
    const textEl = textRefs.current[index];
    if (textEl && textEl.scrollWidth > textEl.clientWidth + 1) {
      event.currentTarget.title = full;
      return;
    }
    event.currentTarget.removeAttribute('title');
  };

  const handleItemDown = (event: PointerEvent<HTMLDivElement>, index: number): void => {
    event.preventDefault();
    if (mobile) {
      return;
    }
    onHover(index);
    onPick(index);
  };

  const handleItemTap = (index: number): void => {
    if (index !== selected) {
      onHover(index);
      return;
    }
    onPick(index);
  };

  const handleInputKeyDown = (event: KeyboardEvent<HTMLInputElement>): void => {
    if (event.key === 'Escape') {
      event.preventDefault();
      onClose();
      return;
    }
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault();
      onMove(event.key === 'ArrowDown' ? 1 : -1);
      return;
    }
    if (event.key === 'Tab') {
      event.preventDefault();
      onAccept();
      return;
    }
    if (event.key === 'Enter') {
      event.preventDefault();
      onSubmit();
    }
  };

  const handleHintDown = (event: PointerEvent<HTMLElement>, action: () => void): void => {
    event.preventDefault();
    action();
  };

  const renderSkeleton = (): ReactNode => {
    return SKELETON_ROWS.map(row => (
      <div key={row} className="tm-suggest-skeleton-row">
        <div className="tm-suggest-skeleton" style={{ animationDelay: `${row * 130}ms` }} />
      </div>
    ));
  };

  const renderList = (): ReactNode => {
    return list.map((item, index) => {
      const isBlock = item.includes('\n');
      const full = `${typed}${item}`;
      return (
        <div
          key={`${index}-${item}`}
          className={resolveItemClass(index, item)}
          style={{ animationDelay: `${Math.min(index, 6) * 26}ms` }}
          onMouseEnter={mobile ? undefined : event => handleItemEnter(event, index, full)}
          onPointerDown={event => handleItemDown(event, index)}
          onClick={mobile ? () => handleItemTap(index) : undefined}
        >
          <div
            ref={node => {
              textRefs.current[index] = node;
            }}
            className="tm-suggest-item-text"
          >
            {isBlock ? (
              <span className="tm-suggest-item-completion">{item}</span>
            ) : (
              <>
                <span className="tm-suggest-item-typed">{typed}</span>
                <span className="tm-suggest-item-completion">{item}</span>
              </>
            )}
          </div>
        </div>
      );
    });
  };

  const renderBody = (): ReactNode => {
    if (loading && list.length === 0) {
      return renderSkeleton();
    }
    if (error.length > 0) {
      return <div className="tm-suggest-empty">{error}</div>;
    }
    if (list.length === 0) {
      return <div className="tm-suggest-empty">{t('protocol.terminal.suggestEmpty')}</div>;
    }
    return renderList();
  };

  const renderMeta = (): ReactNode => {
    if (loading) {
      return <div className="tm-suggest-spinner" />;
    }
    if (list.length === 0) {
      return null;
    }
    return <span className="tm-suggest-count">{list.length}</span>;
  };

  const renderHint = (keys: string, label: string, action: () => void): ReactNode => (
    <div key={keys} className="tm-suggest-hint" onPointerDown={event => handleHintDown(event, action)}>
      <kbd className="tm-suggest-hint-key">{keys}</kbd>
      <span className="tm-suggest-hint-label">{label}</span>
    </div>
  );

  const renderMoveHint = (): ReactNode => (
    <div className="tm-suggest-hint">
      <kbd className="tm-suggest-hint-key" onPointerDown={event => handleHintDown(event, () => onMove(-1))}>
        ↑
      </kbd>
      <kbd className="tm-suggest-hint-key" onPointerDown={event => handleHintDown(event, () => onMove(1))}>
        ↓
      </kbd>
      <span className="tm-suggest-hint-label">{t('protocol.terminal.suggestSelect')}</span>
    </div>
  );

  useEffect(() => {
    inputRef.current?.focus({preventScroll: true});
  }, []);

  return (
    <div className={mobile ? 'tm-suggest-panel is-mobile' : 'tm-suggest-panel'} data-tm-gesture-ignore="true" data-tm-ime="true" style={mobile ? undefined : { width, maxWidth, left: anchor.left, top: anchor.top }}>
      <div className="tm-suggest-head">
        {mobile ? (
          <button type="button" className="tm-suggest-mobile-action" aria-label={t('common.close')} onPointerDown={event => event.preventDefault()} onClick={onClose}>
            <CloseRounded fontSize="small" />
          </button>
        ) : null}
        <input ref={inputRef} value={instruction} className="tm-suggest-input" enterKeyHint="enter" placeholder={t('protocol.terminal.suggestPlaceholder')} onChange={event => onInstructionChange(event.target.value)} onKeyDown={handleInputKeyDown} />
        {mobile ? (
          <>
            <span className="tm-suggest-mobile-meta">{renderMeta()}</span>
            <button type="button" className="tm-suggest-mobile-action is-primary" aria-label={t('protocol.terminal.suggestAccept')} onPointerDown={event => event.preventDefault()} onClick={onSubmit}>
              <ArrowUpwardRounded fontSize="small" />
            </button>
          </>
        ) : null}
      </div>
      <div className="tm-suggest-body">{renderBody()}</div>
      {!mobile ? (
        <div className="tm-suggest-foot">
          <div className="tm-suggest-hints">
            {renderMoveHint()}
            {renderHint('↵', t('protocol.terminal.suggestAccept'), onAccept)}
            {renderHint('Esc', t('common.close'), onClose)}
          </div>
          {renderMeta()}
        </div>
      ) : null}
    </div>
  );
}
