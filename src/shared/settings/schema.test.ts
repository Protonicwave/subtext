import { describe, expect, it } from 'vitest';
import { DEFAULTS, FIELDS, comfortOf, deadKeysIn, settingsFrom, storedAs } from './schema';

describe('the settings a library file holds', () => {
  it('reads back what was written, whatever kind of setting it is', () => {
    for (const [name, value] of [
      ['matching', 'exact'],
      ['subtitleSize', 5.5],
      ['libraryDescending', true],
      ['grain', 0],
      ['subtitleLeadInMs', 120],
      ['subtitleMinimumMs', 0],
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
      { key: FIELDS.libraryDescending.key, value: 'yes' },
      { key: FIELDS.subtitleSize.key, value: 'quite large' },
      { key: FIELDS.subtitleSize.key.replace('size', 'unheard-of'), value: '3' },
    ];

    const settings = settingsFrom(nonsense);

    expect(settings.matching).toBe('relaxed');
    expect(settings.libraryDescending).toBe(DEFAULTS.libraryDescending);
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

  /*
   * Every key a 0.4.0 library file could hold. Two of them described the
   * transcript panel, and five more were intervals the player is now told
   * rather than asked about, so a file written by that release is the case this
   * one has to open without complaint.
   */
  const FROM_0_4_0 = [
    { key: 'appearance.accent', value: 'fixed' },
    { key: 'appearance.glow', value: '0.4' },
    { key: 'appearance.grain', value: '0' },
    { key: 'appearance.motion', value: 'reduced' },
    { key: 'library.matching', value: 'exact' },
    { key: 'library.missing', value: 'hide' },
    { key: 'playback.arrows', value: 'false' },
    { key: 'playback.hardware', value: 'false' },
    { key: 'playback.hide', value: '4000' },
    { key: 'playback.resume', value: 'beginning' },
    { key: 'playback.rewind', value: '12000' },
    { key: 'playback.skip', value: '30000' },
    { key: 'playback.watched', value: '0.9' },
    { key: 'subtitles.background', value: 'panel' },
    { key: 'subtitles.colour', value: '#ffd98a' },
    { key: 'subtitles.language', value: 'fr' },
    { key: 'subtitles.lead', value: '150' },
    { key: 'subtitles.minimum', value: '1200' },
    { key: 'subtitles.position', value: '10' },
    { key: 'subtitles.size', value: '5.5' },
    { key: 'subtitles.typeface', value: 'serif' },
    { key: 'subtitles.weight', value: '600' },
    { key: 'transcript.follow', value: 'false' },
    { key: 'transcript.typeface', value: 'serif' },
  ];

  it('keeps everything a 0.4.0 file said that still means something', () => {
    const settings = settingsFrom(FROM_0_4_0);

    expect(settings.accent).toBe('fixed');
    expect(settings.motion).toBe('reduced');
    expect(settings.matching).toBe('exact');
    expect(settings.missingFilms).toBe('hide');
    expect(settings.resume).toBe('beginning');
    expect(settings.hardwareDecoding).toBe(false);
    expect(settings.subtitleLanguage).toBe('fr');
    expect(settings.subtitleSize).toBe(5.5);
    expect(settings.subtitleLeadInMs).toBe(150);
    expect(settings.subtitleMinimumMs).toBe(1_200);
  });

  it('stands the default in for a setting that release had never heard of', () => {
    // Nothing wrote the cover size before this one, and a library opened for
    // the first time by this build has to be drawn at some size.
    expect(settingsFrom(FROM_0_4_0).tileSize).toBe(DEFAULTS.tileSize);
  });

  it('names the keys from a 0.4.0 file that nothing reads any more', () => {
    expect(deadKeysIn(FROM_0_4_0).sort()).toStrictEqual([
      'playback.arrows',
      'playback.hide',
      'playback.rewind',
      'playback.skip',
      'playback.watched',
      'transcript.follow',
      'transcript.typeface',
    ]);
  });

  it('finds nothing to forget in a file this release wrote', () => {
    const written = Object.keys(FIELDS).map((name) =>
      storedAs(name as keyof typeof FIELDS, DEFAULTS[name as keyof typeof FIELDS]),
    );

    expect(deadKeysIn(written)).toStrictEqual([]);
  });

  it('hands the timeline the two settings it is built from', () => {
    // The mapping lives here rather than in the two places that build a
    // timeline, so that neither of them can be given something the other is not.
    expect(comfortOf(DEFAULTS)).toStrictEqual({
      leadInMs: DEFAULTS.subtitleLeadInMs,
      minimumMs: DEFAULTS.subtitleMinimumMs,
    });
  });
});
