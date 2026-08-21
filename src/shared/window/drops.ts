import { getCurrentWebview } from '@tauri-apps/api/webview';

/** Where a drop landed, in the window's own pixels rather than the screen's. */
export interface DropPoint {
  x: number;
  y: number;
}

interface DropHandlers {
  /** Something is being dragged over the window. */
  over: () => void;
  /** It has left again, or the drop was cancelled. */
  left: () => void;
  /**
   * Files were let go of, at a point in the window.
   *
   * The point is reported because a drop is the window's event and not the
   * element's: the webview says where it happened and leaves working out what
   * was under it to whoever cares.
   */
  drop: (paths: string[], at: DropPoint) => void;
}

/**
 * Files dragged onto the window, with the paths they came from.
 *
 * The browser's own drag and drop is no use here: it hands over file contents,
 * and Subtext wants to know where a file lives so it can watch the folder
 * around it and never copy anything. The webview reports the real paths
 * instead, which is the whole reason this goes through Tauri rather than
 * through drag events on the document.
 */
export function onFilesDropped(handlers: DropHandlers): Promise<() => void> {
  return getCurrentWebview().onDragDropEvent((event) => {
    switch (event.payload.type) {
      case 'enter':
      case 'over':
        handlers.over();
        break;
      case 'drop':
        // The webview reports the position in device pixels and the document
        // is laid out in CSS pixels, which differ on any display scaled above
        // one hundred per cent.
        handlers.drop(event.payload.paths, {
          x: event.payload.position.x / window.devicePixelRatio,
          y: event.payload.position.y / window.devicePixelRatio,
        });
        break;
      default:
        handlers.left();
    }
  });
}
