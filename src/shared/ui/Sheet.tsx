import {
  useEffect,
  useId,
  useRef,
  type KeyboardEvent,
  type ReactNode,
  type SyntheticEvent,
} from 'react';
import styles from './Sheet.module.css';

interface SheetProps {
  open: boolean;
  title: string;
  /** The line under the heading, which is often counts rather than a sentence. */
  description?: ReactNode;
  children: ReactNode;
  footer?: ReactNode;
  /**
   * Whether Escape and a click outside close it. False while something is
   * running that would be left half done, where the way out is the thing
   * finishing rather than the sheet going away.
   */
  dismissible?: boolean;
  onClose: () => void;
}

/**
 * The panel that comes forward over everything else.
 *
 * Built on the platform's own dialog element, which brings the things a
 * hand-rolled overlay gets wrong: focus is trapped inside it, everything behind
 * it stops being reachable by tab or by a screen reader, and it is on the top
 * layer so no stacking context can get above it.
 */
export function Sheet({
  open,
  title,
  description,
  children,
  footer,
  dismissible = true,
  onClose,
}: SheetProps) {
  const dialog = useRef<HTMLDialogElement>(null);
  // Two sheets can be mounted at once, one of them closed, so the heading they
  // are named by cannot be a fixed identifier: both would answer to it and both
  // would take the name of whichever came first.
  const headingId = useId();

  useEffect(() => {
    const element = dialog.current;
    if (!element) return;
    if (open && !element.open) element.showModal();
    if (!open && element.open) element.close();
  }, [open]);

  // Escape is handled here rather than left to the element's own cancel event,
  // so that a sheet which must not be dismissed also cannot be dismissed by
  // the key, and so that both paths out are the one line of code.
  const onKeyDown = (event: KeyboardEvent<HTMLDialogElement>) => {
    if (event.key !== 'Escape') return;
    event.preventDefault();
    if (dismissible) onClose();
  };

  /* A click on the dialog itself is a click on the backdrop: the panel inside
     it covers everything else. */
  const onClick = (event: SyntheticEvent<HTMLDialogElement>) => {
    if (dismissible && event.target === dialog.current) onClose();
  };

  return (
    <dialog
      ref={dialog}
      className={styles.backdrop}
      aria-labelledby={headingId}
      onKeyDown={onKeyDown}
      onClick={onClick}
      onCancel={(event) => {
        event.preventDefault();
      }}
    >
      <div className={styles.sheet}>
        <header className={styles.head}>
          <h2 id={headingId} className={styles.title}>
            {title}
          </h2>
          {description !== undefined && <p className={styles.description}>{description}</p>}
        </header>

        <div className={styles.body}>{children}</div>

        {footer !== undefined && <footer className={styles.foot}>{footer}</footer>}
      </div>
    </dialog>
  );
}
