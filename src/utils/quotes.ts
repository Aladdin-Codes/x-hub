// 本地内置名言金句语料（离线兜底 / local 模式轮换用）。
// 纯通用名言金句，不出网、不依赖任何服务。

export interface LocalQuote {
  content: string
  from: string
}

export const LOCAL_QUOTES: LocalQuote[] = [
  { content: 'Make steady progress, and no effort is ever wasted.', from: 'Proverb' },
  { content: 'The road ahead is long and far; I will search high and low.', from: 'Qu Yuan, “Li Sao”' },
  { content: 'A fine sword is forged through honing; plum blossoms bloom in winter’s bitter cold.', from: 'Admonitions for the Ages' },
  { content: 'A journey of a thousand miles begins beneath your feet.', from: 'Laozi' },
  { content: 'Without small steps, you cannot travel a thousand miles.', from: 'Xunzi, “Encouraging Learning”' },
  { content: 'Keep at it, and even metal and stone can be carved.', from: 'Xunzi, “Encouraging Learning”' },
  { content: 'Skill grows through diligence and withers through play; success comes from thought and falls to carelessness.', from: 'Han Yu, “The Explanation of Learning”' },
  { content: 'Take in widely, choose carefully, build deep reserves, and let your brilliance emerge.', from: 'Su Shi' },
  { content: 'What you gain from books is always shallow until you put it into practice.', from: 'Lu You' },
  { content: 'The wind will one day carry my sails across the sea.', from: 'Li Bai, “Hard Is the Road”' },
  { content: 'I was born with talents that will serve a purpose; when the gold is gone, it will return.', from: 'Li Bai, “Bring in the Wine”' },
  { content: 'One day I will reach the summit and see the mountains laid out below.', from: 'Du Fu, “Gazing at Mount Tai”' },
  { content: 'When mountains and rivers seem to leave no way forward, a village appears beyond the willows and flowers.', from: 'Lu You, “A Visit to the Village West of the Mountain”' },
  { content: 'Beyond the sunken ships, a thousand sails pass; before the dying trees, spring returns.', from: 'Liu Yuxi' },
  { content: 'If you do not strive in your youth, regret will find you in old age.', from: '“A Long Song”' },
  { content: 'Learning without thought is futile; thought without learning is perilous.', from: 'The Analects, “For Government”' },
  { content: 'Review the past to discover the new, and you can be a teacher.', from: 'The Analects' },
  { content: 'The wise are free from doubt, the benevolent from worry, and the brave from fear.', from: 'The Analects' },
  { content: 'Do not impose on others what you do not want for yourself.', from: 'The Analects' },
  { content: 'As Heaven moves with strength, the noble person strives without cease.', from: 'The Book of Changes' },
  { content: 'As Earth is receptive and broad, the noble person carries the world with virtue.', from: 'The Book of Changes' },
  { content: 'Without resolve, wisdom cannot be put into practice.', from: 'Mozi' },
  { content: 'In hardship, cultivate yourself; in prosperity, help the whole world.', from: 'Mencius' },
  { content: 'The old steed rests in its stable, yet still dreams of a thousand-mile journey.', from: 'Cao Cao, “Though the Tortoise Lives Long”' },
]

let lastIndex = -1

/** 随机取一条本地语料，避免与上一条重复 */
export function randomLocalQuote(): LocalQuote {
  if (LOCAL_QUOTES.length === 0) return { content: '', from: '' }
  if (LOCAL_QUOTES.length === 1) return LOCAL_QUOTES[0]
  let idx = Math.floor(Math.random() * LOCAL_QUOTES.length)
  if (idx === lastIndex) idx = (idx + 1) % LOCAL_QUOTES.length
  lastIndex = idx
  return LOCAL_QUOTES[idx]
}
