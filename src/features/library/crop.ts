/**
 * What shape a picture is drawn at.
 *
 * Two answers, because there are two questions. A poster keeps the shape of the
 * picture it was made from and is only bounded in size, which is what [`fitted`]
 * does: artwork is already the shape of a cover, and a frame is sixteen by nine
 * and is letterboxed by whatever draws it rather than cut down here. Taking a
 * rectangle of a given shape out of the middle of a frame is what [`centred`]
 * does, and it stays for the places that want a wide picture out of a frame
 * that is taller than they are.
 */
export interface Crop {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface Size {
  width: number;
  height: number;
}

/**
 * The largest rectangle of a given shape at the middle of a frame.
 *
 * Taking the middle keeps whoever is in the shot. It is no longer how a poster
 * is made: a poster two by three cut out of a frame throws away half the width
 * of the shot before anything has a say in it.
 */
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

/**
 * A picture at its own shape, no wider and no taller than it is allowed to be.
 *
 * The width leads, since a poster is stored at a width that holds up on a dense
 * display. A picture tall enough to pass the second bound leads with its height
 * instead and comes back smaller, which is the shape kept rather than the shape
 * cut: any file at all can be dropped on a film, and a canvas of whatever
 * proportions arrived is not something to allocate on trust.
 */
export function fitted(picture: Size, widest: number, tallest: number): Size {
  if (picture.width <= 0 || picture.height <= 0) return { width: 0, height: 0 };

  const height = Math.round((widest * picture.height) / picture.width);
  if (height <= tallest) return { width: widest, height: Math.max(1, height) };

  return {
    width: Math.max(1, Math.round((tallest * picture.width) / picture.height)),
    height: tallest,
  };
}
