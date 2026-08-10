import { Link, useRoute } from 'wouter'
import { ArrowLeft, ExternalLink, Star } from 'lucide-react'
import { Card, CardContent, CardHeader, CardTitle } from '../components/ui/card'
import { Badge } from '../components/ui/badge'
import { Button } from '../components/ui/button'
import { ShareMenu } from '../components/ShareMenu'
import { useFavorites } from '../hooks/useFavorites'
import { useNews } from '../hooks/useNews'
import { fmtDate } from '../lib/data'

export default function NewsDetail() {
  const [, params] = useRoute('/news/:id')
  const allNews = useNews()
  const item = allNews.find((n) => n.id === params?.id)
  const { toggle, isFavorite } = useFavorites()

  if (!item) {
    return (
      <div className="py-16 text-center">
        <p className="text-muted-foreground">Новость не найдена</p>
        <Link href="/news" className="mt-4 inline-block">
          <Button variant="outline">
            <ArrowLeft size={16} /> К новостям
          </Button>
        </Link>
      </div>
    )
  }

  const related = allNews.filter((n) => n.tag === item.tag && n.id !== item.id)

  return (
    <div className="mx-auto flex max-w-2xl flex-col gap-6">
      <Link href="/news" className="self-start">
        <Button variant="ghost">
          <ArrowLeft size={16} /> Все новости
        </Button>
      </Link>

      <article>
        <div className="flex items-center gap-3">
          <Badge variant="secondary">{item.tag}</Badge>
          <span className="text-sm text-muted-foreground">{fmtDate(item.date)}</span>
        </div>
        <h1 className="mt-3 text-3xl font-bold leading-tight">{item.title}</h1>
        <div className="mt-5 flex flex-col gap-4">
          {item.body.map((p, i) => (
            <p key={i} className="text-lg leading-relaxed text-foreground/90">
              {p}
            </p>
          ))}
        </div>
      </article>

      <div className="flex flex-wrap gap-2">
        <ShareMenu title={item.title} path={`/news/${item.id}`} />
        <Button variant="outline" onClick={() => toggle(item.id)} aria-pressed={isFavorite(item.id)}>
          <Star size={16} className={isFavorite(item.id) ? 'fill-accent text-accent' : ''} />
          {isFavorite(item.id) ? 'В избранном' : 'В избранное'}
        </Button>
      </div>

      <section>
        <h2 className="mb-2 text-sm font-semibold uppercase tracking-wide text-muted-foreground">
          Источники
        </h2>
        <ul className="flex flex-col gap-1.5">
          {item.sources.map((s) => (
            <li key={s.url}>
              <a
                href={s.url}
                target="_blank"
                rel="noreferrer noopener"
                className="inline-flex items-center gap-1.5 text-sm text-secondary hover:underline"
              >
                <ExternalLink size={13} /> {s.label}
              </a>
            </li>
          ))}
        </ul>
      </section>

      {related.length > 0 && (
        <section>
          <h2 className="mb-3 text-lg font-bold">Ещё по теме «{item.tag}»</h2>
          <div className="flex flex-col gap-3">
            {related.map((n) => (
              <Link key={n.id} href={`/news/${n.id}`}>
                <Card className="cursor-pointer transition-colors hover:border-secondary">
                  <CardHeader>
                    <CardTitle className="text-base">{n.title}</CardTitle>
                  </CardHeader>
                  <CardContent>
                    <span className="text-xs text-muted-foreground">{fmtDate(n.date)}</span>
                  </CardContent>
                </Card>
              </Link>
            ))}
          </div>
        </section>
      )}
    </div>
  )
}
