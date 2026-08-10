import { useEffect, useState } from 'react'
import { AI_NEWS, type AiNews } from '../data/report'

/* Новости грузятся с сервера (/data/news.json — его ежедневно обновляет
   YandexGPT-скрипт), а до/вместо него показывается выпуск, зашитый в сборку. */

let cache: AiNews[] | null = null

function normalize(raw: unknown): AiNews[] | null {
  if (!Array.isArray(raw) || raw.length === 0) return null
  const items: AiNews[] = []
  for (const n of raw) {
    if (!n || typeof n !== 'object') return null
    const o = n as Record<string, unknown>
    if (typeof o.id !== 'string' || typeof o.title !== 'string') return null
    items.push({
      id: o.id,
      title: o.title,
      date: typeof o.date === 'string' ? o.date : '',
      tag: typeof o.tag === 'string' ? o.tag : 'Рынок',
      summary: typeof o.summary === 'string' ? o.summary : '',
      body: Array.isArray(o.body) ? o.body.filter((p): p is string => typeof p === 'string') : [],
      sources: Array.isArray(o.sources)
        ? o.sources.filter(
            (s): s is { label: string; url: string } =>
              !!s && typeof (s as Record<string, unknown>).label === 'string' &&
              typeof (s as Record<string, unknown>).url === 'string',
          )
        : [],
    })
  }
  return items
}

export function useNews(): AiNews[] {
  const [news, setNews] = useState<AiNews[]>(cache ?? AI_NEWS)

  useEffect(() => {
    if (cache) return
    fetch('/data/news.json', { cache: 'no-store' })
      .then((r) => (r.ok ? r.json() : null))
      .then((raw) => {
        const items = normalize(raw)
        if (items) {
          cache = items
          setNews(items)
        }
      })
      .catch(() => {
        /* файла ещё нет — остаёмся на встроенном выпуске */
      })
  }, [])

  return news
}
