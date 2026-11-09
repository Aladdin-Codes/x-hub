/**
 * Tests the built-in guide table of contents against docs/user-guide.md:
 *   node --test tests/guideToc.test.mjs
 *
 * Verifies that all 15 chapters are present and ordered, each chapter has a
 * section, and anchors are unique and match the generated table of contents.
 */
import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { countOccurrences, slugifyHeading, splitGuideDocument } from '../src/utils/guideToc.ts'

const root = join(dirname(fileURLToPath(import.meta.url)), '..')
const md = readFileSync(join(root, 'docs/user-guide.md'), 'utf8')
const doc = splitGuideDocument(md)

test('recognizes the document title and introduction', () => {
  assert.equal(doc.title, 'x-hub User Guide')
  assert.match(doc.preamble, /Applies to:/)
  // The introduction must be separated from the first chapter.
  assert.match(doc.preamble, /How to read this guide/)
})

test('extracts all 15 chapters in order without Markdown heading syntax', () => {
  assert.equal(doc.chapters.length, 15)
  assert.deepEqual(
    doc.chapters.map((c) => c.title),
    [
      '1. About x-hub',
      '2. Global Shortcuts',
      '3. Dashboard',
      '4. To-dos',
      '5. Notes',
      '6. Quick Launch',
      '7. AI Assistant',
      '8. Clipboard History',
      '9. Global Search',
      '10. Extension Center',
      '11. Floating Ball and System Tray',
      '12. Settings at a Glance',
      '13. Data and Maintenance',
      '14. App Updates',
      '15. FAQ',
    ],
  )
  for (const c of doc.chapters) assert.ok(c.title.length > 0)
})

test('each chapter contains body text or at least one populated section', () => {
  for (const c of doc.chapters) {
    const hasOwnBody = c.body.length > 0
    const hasSectionBody = c.sections.length > 0 && c.sections.some((s) => s.body.length > 0)
    assert.ok(hasOwnBody || hasSectionBody, `Chapter "${c.title}" has no content.`)
  }
})

test('chapter anchors are unique and support in-page navigation', () => {
  const ids = doc.chapters.flatMap((c) => [c.id, ...c.sections.map((s) => s.id)])
  assert.equal(new Set(ids).size, ids.length)
  for (const id of ids) {
    assert.match(id, /^[a-z0-9\p{L}_-]+$/u, `Invalid anchor format: ${id}`)
  }
})

test('slugifyHeading 与 GitHub 口径一致', () => {
  assert.equal(slugifyHeading('1. 认识 x-hub'), '1-认识-x-hub')
  assert.equal(slugifyHeading('5.8 一键美化与 AI 深度整理'), '58-一键美化与-ai-深度整理')
  assert.equal(slugifyHeading('15. 常见问题'), '15-常见问题')
})

test('四反引号围栏里的伪标题不会被当成章节', () => {
  const sample = ['# T', '', '## A', '', '````md', '## 假标题', '````', '', '## B'].join('\n')
  const parsed = splitGuideDocument(sample)
  assert.deepEqual(parsed.chapters.map((c) => c.title), ['A', 'B'])
})

test('重名标题自动加序号，不产生重复锚点', () => {
  const parsed = splitGuideDocument('# T\n\n## 同一节\n\n## 同一节')
  assert.deepEqual(parsed.chapters.map((c) => c.id), ['同一节', '同一节-2'])
})

test('目录内搜索按关键词计数（大小写不敏感）', () => {
  assert.equal(countOccurrences('Ctrl+K 与 ctrl+k', 'ctrl+k'), 2)
  assert.equal(countOccurrences('没有命中', '壁纸'), 0)
  assert.equal(countOccurrences('任意文本', '   '), 0)
})
