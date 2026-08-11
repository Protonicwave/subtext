import { describe, expect, it } from 'vitest';
import { DEFAULTS, FIELDS, settingsFrom, storedAs } from './schema';

describe('the settings a library file holds', () => {
  it('reads back what was written, whatever kind of setting it is', () => {
    for (const [name, value] of [
      ['matching', 'exact'],
      ['subtitleSize', 5.5],
      ['dialogueArrows', false],
      ['grain', 0],
    ] as const) {
      const written = storedAs(name, value);
      expect(settingsFrom([written])[name]).toBe(value);
    }
  });

  it('stands the defaults in for everything nobody has set', () => {
    const settings = settingsFrom([storedAs('grain', 0.06)]);

    expect(settings.grain).toBe(0.06);
    expect(settings.subtitleSize).toBe(DEFAULTS.subtitleSize);
    expect(settings.motion).toBe('system');
  });

  it('has a default for every setting it knows about', () => {
    for (const name of Object.keys(FIELDS)) {
      expect(DEFAULTS[name as keyof typeof FIELDS]).toBeDefined();
    }
  });

  it('keeps every setting under a key of its own', () => {
    const keys = Object.values(FIELDS).map((field) => field.key);
    expect(new Set(keys).size).toBe(keys.length);
  });

  it('reads a value it does not recognise as the default', () => {
    // How one of these gets there: a file edited by hand, or written by a
    // version that offered something this one does not.
    const nonsense = [
      { key: FIELDS.matching.key, value: 'whatever-comes-next' },
      { key: FIELDS.dialogueArrows.key, value: 'yes' },
      { key: FIELDS.subtitleSize.key, value: 'quite large' },
      { key: FIELDS.subtitleSize.key.replace('size', 'unheard-of'), value: '3' },
    ];

    const settings = settingsFrom(nonsense);

    expect(settings.matching).toBe('relaxed');
    expect(settings.dialogueArrows).toBe(DEFAULTS.dialogueArrows);
    expect(settings.subtitleSize).toBe(DEFAULTS.subtitleSize);
  });

  it('brings a number from outside the range back inside it', () => {
    // The subtitles would otherwise be drawn off the bottom of the picture, or
    // at a size that fills it.
    expect(settingsFrom([{ key: FIELDS.subtitleSize.key, value: '400' }]).subtitleSize).toBe(8);
    expect(settingsFrom([{ key: FIELDS.subtitlePosition.key, value: '-9' }]).subtitlePosition).toBe(
      2,
    );
  });

  it('reads an empty value as no value rather than as nothing at all', () => {
    expect(settingsFrom([{ key: FIELDS.glow.key, value: '' }]).glow).toBe(DEFAULTS.glow);
  });
});
