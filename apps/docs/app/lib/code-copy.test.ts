import { describe, expect, it } from 'vitest';
import { classifyCopiedCode } from './code-copy';

describe('classifyCopiedCode', () => {
  it('detects docker pull', () => {
    expect(
      classifyCopiedCode(
        'docker pull ghcr.io/vinayakkulkarni/tileserver-rs:latest',
      ),
    ).toEqual({ kind: 'docker', command: 'pull' });
  });

  it('detects docker run', () => {
    expect(classifyCopiedCode('docker run -p 8080:8080 tileserver-rs')).toEqual(
      {
        kind: 'docker',
        command: 'run',
      },
    );
  });

  it('detects docker compose in both spellings', () => {
    expect(classifyCopiedCode('docker compose up -d')).toEqual({
      kind: 'docker',
      command: 'compose',
    });
    expect(classifyCopiedCode('docker-compose up')).toEqual({
      kind: 'docker',
      command: 'compose',
    });
  });

  it('detects install commands', () => {
    expect(
      classifyCopiedCode('brew install vinayakkulkarni/tap/tileserver-rs'),
    ).toEqual({ kind: 'install', method: 'homebrew' });
    expect(classifyCopiedCode('cargo install tileserver-rs')).toEqual({
      kind: 'install',
      method: 'cargo',
    });
  });

  it('does not count installs of other tools', () => {
    expect(classifyCopiedCode('brew install ninja ccache libuv')).toEqual({
      kind: 'other',
    });
    expect(classifyCopiedCode('cargo install cargo-insta')).toEqual({
      kind: 'other',
    });
  });

  it('falls back to other', () => {
    expect(classifyCopiedCode('[server]\nport = 8080')).toEqual({
      kind: 'other',
    });
  });
});
