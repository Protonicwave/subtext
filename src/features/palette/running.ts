import { useSettings } from '@/shared/settings/useSettings';
import { useNavigation } from '@/app/routes';
import { useImport } from '@/features/onboarding/useImport';
import type { ActionId } from './commands';

/**
 * What each command actually does.
 *
 * The same call the control on the screen makes, rather than a second way of
 * doing the same thing: choosing a folder is the import store's, the view is a
 * setting, and a rescan is the button in the library toolbar. A command that
 * reimplemented one of those would be a command that drifted from it.
 *
 * Read out of the stores at the moment the command runs rather than subscribed
 * to, so that nothing here redraws the palette when a setting changes
 * underneath it.
 */
export function runCommand(id: ActionId) {
  switch (id) {
    case 'folder':
      void useImport.getState().chooseFolder();
      return;
    case 'view': {
      const settings = useSettings.getState();
      settings.change('libraryView', settings.settings.libraryView === 'list' ? 'covers' : 'list');
      return;
    }
    case 'transcript': {
      const settings = useSettings.getState();
      settings.change('transcript', settings.settings.transcript === 'shown' ? 'hidden' : 'shown');
      return;
    }
    case 'rescan':
      void useImport.getState().reread();
      return;
    case 'settings':
      // The same toggle the title bar's own button makes, so that the key does
      // what pressing it twice looks like it should.
      useNavigation
        .getState()
        .go(
          useNavigation.getState().route.screen === 'settings'
            ? { screen: 'library' }
            : { screen: 'settings' },
        );
  }
}
