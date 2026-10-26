import { describe, expect, it } from 'vitest';
import { slugifyName } from '../slug';

describe('slugifyName', () => {
  it('lower-cases and joins words with single dashes', () => {
    expect(slugifyName('Project Alpha')).toBe('project-alpha');
    expect(slugifyName('  R&D -- Notes!! ')).toBe('r-d-notes');
  });

  it('keeps non-ASCII letters like the server does', () => {
    expect(slugifyName('Équipe été')).toBe('équipe-été');
  });

  it('is empty when nothing alphanumeric remains', () => {
    expect(slugifyName('  ---  ')).toBe('');
    expect(slugifyName('')).toBe('');
  });
});
