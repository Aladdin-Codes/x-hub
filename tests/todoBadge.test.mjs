/**
 * 待办截止徽标与完成时间文案（src/utils/todoSchedule.ts）单测。
 * 重点：跨年补年份——用户反馈「待办显示日期，不是当前年份时要显示具体年份」；
 * 以及已完成条目在日历里的时间说明必须把「截止」和「完成于」分开写。
 *   node --test tests/todoBadge.test.mjs
 */
import test from 'node:test'
import assert from 'node:assert/strict'
import { doneMetaLabel, dueBadge, fmtDueFull, nextRepeatLabel } from '../src/utils/todoSchedule.ts'

const at = (y, m, d, h = 12, min = 0) => new Date(y, m - 1, d, h, min).getTime()
/** 后端 completed_at 的存储形态：本地时刻 → 无时区后缀的 UTC 字符串 */
const serverStr = (d) => d.toISOString().slice(0, 23).replace('T', ' ')

test('同年截止：只显示月日', () => {
  const today = new Date(2026, 9, 8) // 2026-10-08
  assert.deepEqual(dueBadge({ due_at: at(2026, 10, 20, 23, 59) }, today), {
    kind: 'date',
    text: 'Oct 20',
  })
})

test('跨年截止：补年份', () => {
  const today = new Date(2026, 9, 8)
  assert.deepEqual(dueBadge({ due_at: at(2027, 1, 3) }, today), {
    kind: 'date',
    text: 'Jan 3, 2027',
  })
})

test('逾期跨年：补年份', () => {
  const today = new Date(2026, 0, 5) // 2026-01-05
  assert.deepEqual(dueBadge({ due_at: at(2025, 12, 30) }, today), {
    kind: 'over',
    text: 'Overdue Dec 30, 2025',
  })
})

test('逾期同年：仍只显示月日', () => {
  const today = new Date(2026, 5, 20)
  assert.deepEqual(dueBadge({ due_at: at(2026, 5, 1) }, today), {
    kind: 'over',
    text: 'Overdue May 1',
  })
})

test('今天 / 明天：不补年份', () => {
  const today = new Date(2026, 11, 31) // 2026-12-31，明天已跨年
  assert.deepEqual(dueBadge({ due_at: at(2026, 12, 31, 9) }, today), {
    kind: 'today',
    text: 'Today 09:00',
  })
  assert.deepEqual(dueBadge({ due_at: at(2027, 1, 1, 9) }, today), {
    kind: 'tmr',
    text: 'Tomorrow',
  })
})

test('无截止：返回 null', () => {
  assert.equal(dueBadge({ due_at: null }, new Date()), null)
})

test('完成时间说明：有截止的条目标清「截止 … · 完成于 …」', () => {
  const now = new Date(2026, 9, 17, 20, 0)
  assert.equal(
    doneMetaLabel(
      { due_at: at(2026, 10, 16, 18), completed_at: serverStr(new Date(2026, 9, 17, 14, 32)) },
      now,
    ),
    'Due October 16, 2026 18:00 · Completed October 17, 2026 14:32',
  )
})

test('完成时间说明：无截止的标注「原无截止」，落格原因不含糊', () => {
  assert.equal(
    doneMetaLabel(
      { due_at: null, completed_at: serverStr(new Date(2026, 9, 17, 14, 32)) },
      new Date(2026, 9, 17, 20, 0),
    ),
    'Completed October 17, 2026 14:32 (no due date)',
  )
})

test('完成时间说明：旧数据缺完成时间时不编造时刻', () => {
  const now = new Date(2026, 9, 17, 20, 0)
  assert.equal(
    doneMetaLabel({ due_at: at(2026, 10, 16, 18), completed_at: null }, now),
    'Due October 16, 2026 18:00 · Completed',
  )
  assert.equal(doneMetaLabel({ due_at: null, completed_at: null }, now), 'Completed')
})

test('完整截止文案：日期型截止（23:5x）不显示时刻，带时刻的照常显示', () => {
  assert.equal(fmtDueFull(at(2026, 10, 20, 23, 59)), 'October 20, 2026')
  assert.equal(fmtDueFull(at(2026, 10, 20, 9, 5)), 'October 20, 2026 09:05')
})

test('周期滚动文案：今天/明天直接说，其余给月日（跨年补年份），无截止说「下一轮」', () => {
  const now = new Date(2026, 9, 17, 20, 0)
  assert.equal(nextRepeatLabel({ due_at: at(2026, 10, 17, 23, 59) }, now), 'Today')
  assert.equal(nextRepeatLabel({ due_at: at(2026, 10, 18, 9, 30) }, now), 'Tomorrow')
  assert.equal(nextRepeatLabel({ due_at: at(2026, 10, 25, 9, 30) }, now), 'Oct 25')
  assert.equal(nextRepeatLabel({ due_at: at(2027, 1, 3, 9, 30) }, now), 'Jan 3, 2027')
  assert.equal(nextRepeatLabel({ due_at: null }, now), 'next occurrence')
})
