/**
 * The largest rectangle of a given shape at the middle of a frame.
 *
 * A film is sixteen by nine and a poster is two by three, so something has to
 * go. Taking the middle keeps whoever is in the shot, where letterboxing the
 * whole frame into a tall tile would give a wall of black bars down both sides
 * of every film in the library.
 */
export interface Crop {
  x: number;
  y: number;
  width: number;
  height: number;
}

export function centred(width: number, height: number, ratio: number): Crop {
  if (width <= 0 || height <= 0 || ratio <= 0) return { x: 0, y: 0, width: 0, height: 0 };

  const wanted = Math.min(width, height * ratio);

  return {
    x: Math.round((width - wanted) / 2),
    y: Math.round((height - wanted / ratio) / 2),
    width: Math.round(wanted),
    height: Math.round(wanted / ratio),
  };
}
