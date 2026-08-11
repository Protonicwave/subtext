import { describe, expect, it, vi } from 'vitest';

vi.mock('./bindings', () => ({
  commands: {
    listFolders: vi.fn(() => Promise.resolve({ status: 'ok', data: ['a folder'] })),
    rescan: vi.fn(() =>
      Promise.resolve({ status: 'error', error: { message: 'the library is locked' } }),
    ),
    addFolder: vi.fn((path: string) => Promise.resolve({ status: 'ok', data: path })),
  },
}));

const { ipc, IpcError, reasonFor, unwrap } = await import('./client');

describe('the command client', () => {
  it('hands back the value a command answered with', async () => {
    await expect(ipc.listFolders()).resolves.toEqual(['a folder']);
  });

  it('passes arguments through to the command', async () => {
    await expect(ipc.addFolder('/films')).resolves.toBe('/films');
  });

  it('throws what the back end said went wrong', async () => {
    await expect(ipc.rescan()).rejects.toThrow('the library is locked');
    await expect(ipc.rescan()).rejects.toBeInstanceOf(IpcError);
  });

  it('unwraps an answer on its own', () => {
    expect(unwrap({ status: 'ok', data: 3 })).toBe(3);
    expect(() => unwrap({ status: 'error', error: { message: 'no' } })).toThrow('no');
  });

  it('finds a sentence for anything that failed', () => {
    expect(reasonFor(new IpcError('the folder is gone'))).toBe('the folder is gone');
    expect(reasonFor(new Error('something else'))).toBe('something else');
    expect(reasonFor('a bare string')).toBe('a bare string');
  });
});
