import { describe, expect, it } from 'vitest';
import { dayLabel, formatClock, humanRemaining, mmss } from './countdown';

const at = (h: number, m: number, s = 0) => {
  const d = new Date(2026, 9, 11, h, m, s);
  return d;
};

describe('formatClock', () => {
  it('pads hours and minutes', () => {
    expect(formatClock(at(1, 0))).toBe('01:00');
    expect(formatClock(at(23, 5))).toBe('23:05');
  });
});

describe('humanRemaining', () => {
  it('formats hours, minutes, and seconds', () => {
    expect(humanRemaining((2 * 3600 + 47 * 60) * 1000)).toBe('2 h 47 m');
    expect(humanRemaining((9 * 60 + 32) * 1000)).toBe('9 min 32 s');
    expect(humanRemaining(42_000)).toBe('42 s');
    expect(humanRemaining(-5)).toBe('0 s');
  });
});

describe('mmss', () => {
  it('formats auto-dismiss countdowns', () => {
    expect(mmss((8 * 60 + 12) * 1000)).toBe('8:12');
    expect(mmss(5_000)).toBe('0:05');
  });
});

describe('dayLabel', () => {
  it('distinguishes today from tomorrow', () => {
    expect(dayLabel(at(23, 0), at(22, 13))).toBe('today');
    expect(dayLabel(new Date(2026, 9, 12, 1, 0), at(22, 13))).toBe('tomorrow');
  });
});

