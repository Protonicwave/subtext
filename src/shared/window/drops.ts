import { getCurrentWebview } from '@tauri-apps/api/webview';

interface DropHandlers {
  /** Something is being dragged over the window. */
  over: () => void;
  /** It has left again, or the drop was cancelled. */
  left: () => void;
  drop: (paths: string[]) => void;
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
        handlers.drop(event.payload.paths);
        break;
      default:
        handlers.left();
    }
  });
}
