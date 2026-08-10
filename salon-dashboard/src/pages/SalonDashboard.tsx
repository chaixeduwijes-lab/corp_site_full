import { useMemo, useState } from 'react'
import {
  Bar, BarChart, CartesianGrid, Cell, ResponsiveContainer, Tooltip, XAxis, YAxis,
} from 'recharts'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../components/ui/card'
import { Badge } from '../components/ui/badge'
import { StatCard } from '../components/StatCard'
import {
  MONTH_SHORT, PAY_LABELS, PROGRAMS, SERIES_COLORS,
  entries, fmtDate, monthLabel, monthsInData, news, num, pluralRu, rub, settings, total,
} from '../lib/data'

const PERIODS = [
  { key: 'all', label: 'Весь период' },
  ...monthsInData.map((m) => ({ key: m, label: monthLabel(m) })),
]

export default function SalonDashboard() {
  const [period, setPeriod] = useState('all')

  const rows = useMemo(
    () => entries.filter((e) => period === 'all' || e.date.slice(0, 7) === period),
    [period],
  )

  const revenue = rows.reduce((a, e) => a + total(e), 0)
  const extras = rows.reduce((a, e) => a + e.extras, 0)
  const newGuests = rows.filter((e) => e.newGuest).length
  const vip = rows.filter((e) => e.vip).length
  const avg = rows.length ? revenue / rows.length : 0

  const goalMonth = period === 'all' ? monthsInData[monthsInData.length - 1] : period
  const goalRevenue = entries
    .filter((e) => e.date.slice(0, 7) === goalMonth)
    .reduce((a, e) => a + total(e), 0)
  const goalPct = Math.round((goalRevenue / settings.monthGoal) * 100)

  const monthly = monthsInData.map((m) => {
    const mr = rows.filter((e) => e.date.slice(0, 7) === m)
    return {
      month: MONTH_SHORT[Number(m.slice(5)) - 1],
      ym: m,
      revenue: mr.reduce((a, e) => a + total(e), 0),
      visits: mr.length,
    }
  })

  const byProgram = PROGRAMS.map((p) => {
    const pr = rows.filter((e) => e.program === p)
    return { program: p, revenue: pr.reduce((a, e) => a + total(e), 0), visits: pr.length }
  })

  const byPay = (Object.keys(PAY_LABELS) as (keyof typeof PAY_LABELS)[]).map((k) => {
    const pr = rows.filter((e) => e.pay === k)
    return { pay: PAY_LABELS[k], revenue: pr.reduce((a, e) => a + total(e), 0), visits: pr.length }
  })

  return (
    <div className="flex flex-col gap-8">
      <section className="flex flex-wrap items-end justify-between gap-4">
        <div>
          <h1 className="text-3xl font-bold text-foreground">Салон — панель показателей</h1>
          <p className="mt-1 text-muted-foreground">
            {num(entries.length)} визитов · 1 января — 6 августа 2026
          </p>
        </div>
        <div className="flex flex-wrap gap-1.5" role="group" aria-label="Период">
          {PERIODS.map((p) => (
            <button
              key={p.key}
              onClick={() => setPeriod(p.key)}
              aria-pressed={period === p.key}
              className={`rounded-full border px-3 py-1 text-sm font-medium transition-colors ${
                period === p.key
                  ? 'border-secondary bg-secondary/15 text-foreground'
                  : 'border-border bg-card text-muted-foreground hover:text-foreground'
              }`}
            >
              {p.label}
            </button>
          ))}
        </div>
      </section>

      <section>
        <h2 className="mb-4 text-xl font-bold">Основные показатели</h2>
        <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-4">
          <StatCard label="Выручка" value={rub(revenue)} note={`допродажи ${rub(extras)}`} />
          <StatCard
            label="Визиты"
            value={num(rows.length)}
            note={`${vip} VIP-${pluralRu(vip, 'визит', 'визита', 'визитов')}`}
          />
          <StatCard label="Средний чек" value={rub(avg)} note="услуги + допродажи" />
          <StatCard
            label="Новые гости"
            value={num(newGuests)}
            trend={{
              dir: 'up',
              good: true,
              text: rows.length ? `${Math.round((newGuests / rows.length) * 100)} % визитов` : '—',
            }}
          />
        </div>
      </section>

      <Card>
        <CardHeader className="flex-row flex-wrap items-baseline justify-between gap-2">
          <div>
            <CardTitle>Цель месяца — {monthLabel(goalMonth).toLowerCase()}</CardTitle>
            <CardDescription>по всем мастерам и программам</CardDescription>
          </div>
          <p className="text-xl font-bold">
            {rub(goalRevenue)}{' '}
            <span className="text-sm font-normal text-muted-foreground">
              из {rub(settings.monthGoal)}
            </span>{' '}
            <span className={goalPct >= 100 ? 'text-secondary' : 'text-accent'}>· {goalPct} %</span>
          </p>
        </CardHeader>
        <CardContent>
          <div className="h-2.5 overflow-hidden rounded-full bg-muted">
            <div
              className="h-full rounded-full bg-secondary"
              style={{ width: `${Math.min(100, goalPct)}%` }}
            />
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Выручка по месяцам</CardTitle>
          <CardDescription>услуги + допродажи; август — по 6-е число</CardDescription>
        </CardHeader>
        <CardContent className="h-72">
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
                formatter={(v) => [rub(Number(v)), 'Выручка']}
                labelFormatter={(_, payload) =>
                  payload?.[0] ? `${monthLabel(payload[0].payload.ym)} · ${payload[0].payload.visits} визитов` : ''
                }
              />
              <Bar dataKey="revenue" radius={[4, 4, 0, 0]} maxBarSize={36}>
                {monthly.map((m) => (
                  <Cell key={m.ym} fill={m.ym === goalMonth && period === 'all' ? '#f59e0b' : '#10b981'} />
                ))}
              </Bar>
            </BarChart>
          </ResponsiveContainer>
        </CardContent>
      </Card>

      <section className="grid grid-cols-1 gap-4 lg:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle>Программы</CardTitle>
            <CardDescription>выручка и визиты по программам</CardDescription>
          </CardHeader>
          <CardContent className="h-64">
            <ResponsiveContainer width="100%" height="100%">
              <BarChart data={byProgram} layout="vertical" margin={{ top: 0, right: 16, left: 8, bottom: 0 }}>
                <CartesianGrid stroke="var(--chart-grid)" horizontal={false} />
                <XAxis type="number" hide />
                <YAxis
                  type="category"
                  dataKey="program"
                  stroke="var(--chart-axis)"
                  tickLine={false}
                  axisLine={false}
                  width={86}
                />
                <Tooltip
                  cursor={{ fill: 'var(--muted)' }}
                  contentStyle={{
                    background: 'var(--card)',
                    border: '1px solid var(--border)',
                    borderRadius: 8,
                    color: 'var(--foreground)',
                  }}
                  formatter={(v) => [rub(Number(v)), 'Выручка']}
                  labelFormatter={(label, payload) =>
                    payload?.[0] ? `${label} · ${payload[0].payload.visits} визитов` : String(label)
                  }
                />
                <Bar dataKey="revenue" radius={[0, 4, 4, 0]} maxBarSize={22}>
                  {byProgram.map((p) => (
                    <Cell key={p.program} fill={SERIES_COLORS[p.program]} />
                  ))}
                </Bar>
              </BarChart>
            </ResponsiveContainer>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Способы оплаты</CardTitle>
            <CardDescription>выручка по способу оплаты</CardDescription>
          </CardHeader>
          <CardContent className="h-64">
            <ResponsiveContainer width="100%" height="100%">
              <BarChart data={byPay} layout="vertical" margin={{ top: 0, right: 16, left: 8, bottom: 0 }}>
                <CartesianGrid stroke="var(--chart-grid)" horizontal={false} />
                <XAxis type="number" hide />
                <YAxis
                  type="category"
                  dataKey="pay"
                  stroke="var(--chart-axis)"
                  tickLine={false}
                  axisLine={false}
                  width={86}
                />
                <Tooltip
                  cursor={{ fill: 'var(--muted)' }}
                  contentStyle={{
                    background: 'var(--card)',
                    border: '1px solid var(--border)',
                    borderRadius: 8,
                    color: 'var(--foreground)',
                  }}
                  formatter={(v) => [rub(Number(v)), 'Выручка']}
                  labelFormatter={(label, payload) =>
                    payload?.[0] ? `${label} · ${payload[0].payload.visits} визитов` : String(label)
                  }
                />
                <Bar dataKey="revenue" fill="#10b981" radius={[0, 4, 4, 0]} maxBarSize={22} />
              </BarChart>
            </ResponsiveContainer>
          </CardContent>
        </Card>
      </section>

      <section>
        <h2 className="mb-4 text-xl font-bold">Новости для салона</h2>
        <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
          {news.map((n) => (
            <Card key={n.id} className="h-full">
              <CardHeader>
                <div className="flex items-center justify-between gap-3">
                  <Badge variant="secondary">{n.tag}</Badge>
                  <span className="text-xs text-muted-foreground">{fmtDate(n.date)}</span>
                </div>
                <CardTitle className="pt-1">{n.title}</CardTitle>
              </CardHeader>
              <CardContent>
                <p className="text-sm text-muted-foreground">{n.text}</p>
              </CardContent>
            </Card>
          ))}
        </div>
      </section>
    </div>
  )
}
