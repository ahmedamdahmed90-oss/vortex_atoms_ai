import { describe, it, expect } from 'vitest'
import { cn, formatTime, formatUptime, formatBytes } from '../helpers'

describe('cn', () => {
  it('joins conditional classes', () => {
    // eslint-disable-next-line eslint/no-constant-binary-expression
    expect(cn('a', false && 'b', 'c')).toBe('a c')
  })

  it('lets later tailwind classes win conflicts', () => {
    expect(cn('px-2', 'px-4')).toBe('px-4')
  })
})

describe('formatUptime', () => {
  it('formats minutes only', () => {
    expect(formatUptime(45 * 60)).toBe('45د')
    expect(formatUptime(30)).toBe('0د')
  })

  it('formats hours and minutes', () => {
    expect(formatUptime(2 * 3600 + 30 * 60)).toBe('2س 30د')
  })

  it('formats days, hours and minutes', () => {
    expect(formatUptime(86400 + 3600 + 60)).toBe('1يوم 1س 1د')
  })
})

describe('formatBytes', () => {
  it('handles zero', () => {
    expect(formatBytes(0)).toBe('0 B')
  })

  it('formats KB and fractional values', () => {
    expect(formatBytes(1024)).toBe('1 KB')
    expect(formatBytes(1536)).toBe('1.5 KB')
  })

  it('formats MB', () => {
    expect(formatBytes(1024 * 1024 * 7.34)).toMatch(/^7\.34 MB$/)
  })
})

describe('formatTime', () => {
  it('returns a hh:mm shaped string (latin or arabic-indic digits)', () => {
    const out = formatTime(new Date(2026, 0, 1, 9, 7).getTime())
    expect(out).toMatch(/[\d\u0660-\u0669]{1,2}:[\d\u0660-\u0669]{2}/)
  })
})
