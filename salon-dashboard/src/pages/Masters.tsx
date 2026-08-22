import {
  Bar, BarChart, CartesianGrid, Legend, ResponsiveContainer, Tooltip, XAxis, YAxis,
} from 'recharts'
import { Star } from 'lucide-react'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../components/ui/card'
import { Badge } from '../components/ui/badge'
import {
  MONTH_SHORT, entries, masters, monthsInData, num, rub, total,
} from '../lib/data'

export default function Masters() {
  const stats = masters.map((m) => {
    const mr = entries.filter((e) => e.master === m.id)
    const revenue = mr.reduce((a, e) => a + total(e), 0)
    return {
      ...m,
      revenue,
      visits: mr.length,
      avg: mr.length ? revenue / mr.length : 0,
      payout: (revenue * m.percent) / 100,
      vip: mr.filter((e) => e.vip).length,
    }
  })
  const best = [...stats].sort((a, b) => b.revenue - a.revenue)[0]

  const monthly = monthsInData.map((ym) => {
    const row: Record<string, number | string> = { month: MONTH_SHORT[Number(ym.slice(5)) - 1] }
    for (const m of masters) {
      row[m.name] = entries
        .filter((e) => e.master === m.id && e.date.slice(0, 7) === ym)
        .reduce((a, e) => a + total(e), 0)
    }
    return row
  })

  const MASTER_COLORS = ['#3b82f6', '#10b981', '#f59e0b']

  return (
    <div className="flex flex-col gap-6">
      <div>
        <h1 className="text-3xl font-bold">Сравнение мастеров</h1>
        <p className="mt-1 text-muted-foreground">
          Выручка, визиты и вознаграждение за весь период
        </p>
      </div>

      <div className="grid grid-cols-1 gap-4 md:grid-cols-3">
        {stats.map((m) => (
          <Card key={m.id} className={m.id === best.id ? 'border-secondary' : undefined}>
            <CardHeader>
              <div className="flex items-center justify-between">
                <CardTitle className="text-lg">{m.name}</CardTitle>
                {m.id === best.id && <Badge variant="secondary">лидер</Badge>}
              </div>
              <CardDescription className="inline-flex items-center gap-1">
                <Star size={14} className="fill-accent text-accent" /> {m.rating.toLocaleString('ru-RU')}
                <span className="mx-1">·</span> ставка {m.percent} %
              </CardDescription>
            </CardHeader>
            <CardContent>
              <p className="text-2xl font-bold">{rub(m.revenue)}</p>
              <dl className="mt-3 grid grid-cols-2 gap-x-3 gap-y-1.5 text-sm">
                <dt className="text-muted-foreground">Визиты</dt>
                <dd className="text-right font-medium">{num(m.visits)}</dd>
                <dt className="text-muted-foreground">Средний чек</dt>
                <dd className="text-right font-medium">{rub(m.avg)}</dd>
                <dt className="text-muted-foreground">VIP-визиты</dt>
                <dd className="text-right font-medium">{m.vip}</dd>
                <dt className="text-muted-foreground">К выплате</dt>
                <dd className="text-right font-semibold text-secondary">{rub(m.payout)}</dd>
              </dl>
            </CardContent>
          </Card>
        ))}
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Выручка мастеров по месяцам</CardTitle>
          <CardDescription>услуги + допродажи, ₽</CardDescription>
        </CardHeader>
        <CardContent className="h-80">
          <ResponsiveContainer width="100%" height="100%">
            <BarChart data={monthly} margin={{ top: 8, right: 8, left: 8, bottom: 0 }}>
              <CartesianGrid stroke="var(--chart-grid)" vertical={false} />
              <XAxis dataKey="month" stroke="var(--chart-axis)" tickLine={false} axisLine={false} />
              <YAxis
                stroke="var(--chart-axis)"
                tickLine={false}
                axisLine={false}
                tickFormatter={(v: number) => `${Math.round(v / 1000)} т.`}
                width={56}
              />
              <Tooltip
                cursor={{ fill: 'var(--muted)' }}
                contentStyle={{
                  background: 'var(--card)',
                  border: '1px solid var(--border)',
                  borderRadius: 8,
                  color: 'var(--foreground)',
                }}
                formatter={(v, name) => [rub(Number(v)), String(name)]}
              />
              <Legend formatter={(value) => <span style={{ color: 'var(--foreground)' }}>{String(value)}</span>} />
              {masters.map((m, i) => (
                <Bar
                  key={m.id}
                  dataKey={m.name}
                  fill={MASTER_COLORS[i % MASTER_COLORS.length]}
                  radius={[4, 4, 0, 0]}
                  maxBarSize={18}
                />
              ))}
            </BarChart>
          </ResponsiveContainer>
        </CardContent>
      </Card>
    </div>
  )
}
