import { type CSSProperties, type RefObject, useEffect, useRef } from 'react';
import { clockOf } from '@/shared/media/clock';
import styles from './ScrubberPreview.module.css';

/**
 * What is at the moment under the pointer.
 *
 * Two answers to the same question. The frame is what every player shows, and
 * it is the one that says which scene this is. The line is the one only a
 * player that holds the dialogue as data can show, and it is usually the more
 * useful of the two: a face in a dark frame says less than what that face is
 * saying.
 *
 * The frame comes from a second element opened on the same file, which exists
 * only while the pointer is over the bar. Holding a decoder open on a four
 * gigabyte film for the whole of it, to answer a question nobody is asking, is
 * not worth the memory.
 */

interface ScrubberPreviewProps {
  /** The film, through the protocol that serves it. */
  source: string;
  atMs: number;
  /** How far along the bar the pointer is, from zero to one. */
  along: number;
  /** What is said around that moment, where anything is. */
  line: string | null;
}

/**
 * How far from a moment a line still counts as being spoken at it.
 *
 * Most of a film is silence, and a preview that said nothing four seconds
 * either side of every line would say nothing most of the time. Wide enough to
 * catch the pauses within an exchange, and not so wide as to answer with a line
 * from the scene before.
 */
export const NEAR_ENOUGH = 4_000;

export function ScrubberPreview({ source, atMs, along, line }: ScrubberPreviewProps) {
  const frame = useFrameAt(atMs);

  return (
    <figure className={styles.preview} style={{ '--along': along } as CSSProperties}>
      <video
        ref={frame}
        className={styles.frame}
        src={source}
        muted
        preload="metadata"
        playsInline
      />
      <figcaption className={styles.caption}>
        <span className={styles.at}>{clockOf(atMs)}</span>
        {line !== null && <span className={styles.line}>{line}</span>}
      </figcaption>
    </figure>
  );
}

/**
 * Puts the frame at a moment, one seek at a time.
 *
 * A pointer crossing the bar asks for a new moment sixty times a second, and a
 * seek in a large file takes longer than that. So a seek is asked for only when
 * the last one has landed, and what is asked for is wherever the pointer has
 * got to by then rather than the queue of everywhere it has been. That is the
 * difference between a preview that keeps up and one that is a second behind
 * the pointer for as long as it moves.
 */
/** There is no element to ask yet, or there is no longer one. */
function nothing() {
  return;
}

function useFrameAt(atMs: number): RefObject<HTMLVideoElement | null> {
  const video = useRef<HTMLVideoElement>(null);
  /** Where the pointer has got to, which is the only moment worth seeking to. */
  const wanted = useRef(atMs);
  /** Asks the element to catch up, once there is an element to ask. */
  const catchUp = useRef<() => void>(nothing);

  useEffect(() => {
    const element = video.current;
    if (element === null) return;

    let seeking = false;
    const ask = () => {
      // Nothing to seek in yet: the file has not said how long it is. Asked
      // again the moment it does.
      if (seeking || element.readyState < HTMLMediaElement.HAVE_METADATA) return;

      const at = wanted.current / 1000;
      // Already there, near enough that a seek would show the same frame.
      if (Math.abs(element.currentTime - at) < 0.05) return;

      seeking = true;
      element.currentTime = at;
    };

    const landed = () => {
      seeking = false;
      ask();
    };

    catchUp.current = ask;
    element.addEventListener('seeked', landed);
    element.addEventListener('loadedmetadata', ask);
    ask();

    return () => {
      catchUp.current = nothing;
      element.removeEventListener('seeked', landed);
      element.removeEventListener('loadedmetadata', ask);
    };
  }, []);

  useEffect(() => {
    wanted.current = atMs;
    catchUp.current();
  }, [atMs]);

  return video;
}
