#!/usr/bin/env node
/* Ежедневное обновление новостного канала через YandexGPT.
   1. Забирает свежие заголовки из RSS-лент об ИИ.
   2. Просит YandexGPT собрать из них русскоязычный дайджест в формате news.json.
   3. Валидирует ответ и пишет файл (по умолчанию salon-dashboard/public/data/news.json).

   Использование:
     YC_API_KEY=... YC_FOLDER_ID=... node scripts/update-news.mjs [путь-к-news.json]

   YC_API_KEY   — API-ключ сервисного аккаунта с ролью ai.languageModels.user
   YC_FOLDER_ID — идентификатор каталога Яндекс Клауда */

import { mkdir, writeFile } from 'node:fs/promises'
import { dirname, resolve } from 'node:path'

const FEEDS = [
  'https://techcrunch.com/category/artificial-intelligence/feed/',
  'https://venturebeat.com/category/ai/feed/',
  'https://www.artificialintelligence-news.com/feed/',
]
const MAX_ITEMS_PER_FEED = 12
const TAGS = ['Релизы', 'Рынок', 'Безопасность', 'Регулирование']

const apiKey = process.env.YC_API_KEY
const folderId = process.env.YC_FOLDER_ID
if (!apiKey || !folderId) {
  console.error('Нужны переменные окружения YC_API_KEY и YC_FOLDER_ID')
  process.exit(1)
}
const outPath = resolve(process.argv[2] ?? 'salon-dashboard/public/data/news.json')

function decodeEntities(s) {
  return s
    .replace(/<!\[CDATA\[(.*?)\]\]>/gs, '$1')
    .replace(/&amp;/g, '&').replace(/&lt;/g, '<').replace(/&gt;/g, '>')
    .replace(/&quot;/g, '"').replace(/&#0?39;/g, "'").replace(/&nbsp;/g, ' ')
    .trim()
}

async function fetchFeed(url) {
  try {
    const res = await fetch(url, {
      headers: { 'user-agent': 'vest-smr-news-bot/1.0' },
      signal: AbortSignal.timeout(20000),
    })
    if (!res.ok) throw new Error(`HTTP ${res.status}`)
    const xml = await res.text()
    const items = []
    for (const m of xml.matchAll(/<item[\s>][\s\S]*?<\/item>/g)) {
      const block = m[0]
      const title = block.match(/<title[^>]*>([\s\S]*?)<\/title>/)?.[1]
      const link = block.match(/<link[^>]*>([\s\S]*?)<\/link>/)?.[1]
      const date = block.match(/<pubDate[^>]*>([\s\S]*?)<\/pubDate>/)?.[1]
      if (title && link) {
        items.push({
          title: decodeEntities(title),
          link: decodeEntities(link),
          date: date ? new Date(decodeEntities(date)).toISOString().slice(0, 10) : '',
        })
      }
      if (items.length >= MAX_ITEMS_PER_FEED) break
    }
    console.error(`✓ ${url}: ${items.length} материалов`)
    return items
  } catch (e) {
    console.error(`✗ ${url}: ${e.message}`)
    return []
  }
}

async function askYandexGpt(headlines) {
  const today = new Date().toISOString().slice(0, 10)
  const prompt = `Ты — редактор новостного канала о технологиях ИИ. Ниже — свежие заголовки из зарубежных СМИ со ссылками. Составь из них дайджест из 6–8 самых важных новостей на русском языке.

Ответь СТРОГО JSON-массивом без пояснений и без markdown. Каждый элемент:
{
  "id": "d${today.replaceAll('-', '')}-1" (порядковый номер),
  "date": "${today}",
  "tag": один из: ${TAGS.map((t) => `"${t}"`).join(', ')},
  "title": "заголовок на русском",
  "summary": "1–2 предложения сути",
  "body": ["абзац 1", "абзац 2"] — пересказ на основе заголовка, без выдуманных фактов и цифр,
  "sources": [{"label": "название издания и заголовок", "url": "ссылка из списка"}]
}

Правила: используй только факты из заголовков; не выдумывай цифры, компании и цитаты; url бери только из списка ниже; каждая новость — минимум один источник.

Заголовки:
${headlines.map((h, i) => `${i + 1}. [${h.date}] ${h.title} — ${h.link}`).join('\n')}`

  const res = await fetch('https://llm.api.cloud.yandex.net/foundationModels/v1/completion', {
    method: 'POST',
    headers: {
      'Authorization': `Api-Key ${apiKey}`,
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({
      modelUri: `gpt://${folderId}/yandexgpt/latest`,
      completionOptions: { stream: false, temperature: 0.3, maxTokens: 4000 },
      messages: [{ role: 'user', text: prompt }],
    }),
    signal: AbortSignal.timeout(120000),
  })
  if (!res.ok) throw new Error(`YandexGPT HTTP ${res.status}: ${await res.text()}`)
  const data = await res.json()
  const text = data?.result?.alternatives?.[0]?.message?.text
  if (!text) throw new Error('Пустой ответ YandexGPT')
  return text
}

function parseNews(text, allowedUrls) {
  const jsonText = text.replace(/^```(?:json)?/m, '').replace(/```\s*$/m, '').trim()
  const start = jsonText.indexOf('[')
  const end = jsonText.lastIndexOf(']')
  if (start === -1 || end === -1) throw new Error('В ответе нет JSON-массива')
  const raw = JSON.parse(jsonText.slice(start, end + 1))
  if (!Array.isArray(raw) || raw.length === 0) throw new Error('Пустой дайджест')

  return raw
    .filter((n) => n && typeof n.id === 'string' && typeof n.title === 'string')
    .map((n, i) => ({
      id: String(n.id || `d${i + 1}`),
      date: typeof n.date === 'string' ? n.date : new Date().toISOString().slice(0, 10),
      tag: TAGS.includes(n.tag) ? n.tag : 'Рынок',
      title: String(n.title).slice(0, 200),
      summary: String(n.summary ?? '').slice(0, 500),
      body: (Array.isArray(n.body) ? n.body : [])
        .filter((p) => typeof p === 'string')
        .map((p) => p.slice(0, 2000)),
      // защита от галлюцинаций: оставляем только ссылки из реальных лент
      sources: (Array.isArray(n.sources) ? n.sources : []).filter(
        (s) => s && typeof s.url === 'string' && allowedUrls.has(s.url) && typeof s.label === 'string',
      ),
    }))
    .filter((n) => n.sources.length > 0)
}

const headlines = (await Promise.all(FEEDS.map(fetchFeed))).flat()
if (headlines.length < 5) {
  console.error(`Слишком мало материалов из лент (${headlines.length}) — файл не обновляю`)
  process.exit(1)
}

const answer = await askYandexGpt(headlines)
const news = parseNews(answer, new Set(headlines.map((h) => h.link)))
if (news.length < 3) {
  console.error(`После валидации осталось ${news.length} новостей — файл не обновляю`)
  process.exit(1)
}

await mkdir(dirname(outPath), { recursive: true })
await writeFile(outPath, JSON.stringify(news, null, 2) + '\n')
console.error(`✓ Записано ${news.length} новостей в ${outPath}`)
