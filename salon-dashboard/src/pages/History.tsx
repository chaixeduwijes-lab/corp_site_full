import { useState } from 'react'
import {
  CartesianGrid, Legend, Line, LineChart, ResponsiveContainer, Tooltip, XAxis, YAxis,
} from 'recharts'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../components/ui/card'
import {
  MONTH_SHORT, PROGRAMS, SERIES_COLORS, entries, monthsInData, rub, total,
} from '../lib/data'

export default function History() {
  const [mode, setMode] = useState<'total' | 'programs'>('total')

  const data = monthsInData.map((ym) => {
    const mr = entries.filter((e) => e.date.slice(0, 7) === ym)
    const row: Record<string, number | string> = {
      month: MONTH_SHORT[Number(ym.slice(5)) - 1],
      'Всего': mr.reduce((a, e) => a + total(e), 0),
    }
    for (const p of PROGRAMS) {
      row[p] = mr.filter((e) => e.program === p).reduce((a, e) => a + total(e), 0)
    }
    return row
  })

  const lines = mode === 'total' ? ['Всего'] : [...PROGRAMS]

  return (
    <div className="flex flex-col gap-6">
      <div className="flex flex-wrap items-end justify-between gap-4">
        <div>
          <h1 className="text-3xl font-bold">История выручки</h1>
          <p className="mt-1 text-muted-foreground">Динамика по месяцам, январь — август 2026</p>
        </div>
        <div className="flex gap-1.5" role="group" aria-label="Режим графика">
          {(
            [
              ['total', 'Всего'],
              ['programs', 'По программам'],
            ] as const
          ).map(([key, label]) => (
            <button
              key={key}
              onClick={() => setMode(key)}
              aria-pressed={mode === key}
              className={`rounded-full border px-3 py-1 text-sm font-medium transition-colors ${
                mode === key
                  ? 'border-secondary bg-secondary/15 text-foreground'
                  : 'border-border bg-card text-muted-foreground hover:text-foreground'
              }`}
            >
              {label}
            </button>
          ))}
        </div>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>{mode === 'total' ? 'Общая выручка' : 'Выручка по программам'}</CardTitle>
          <CardDescription>услуги + допродажи; август — по 6-е число</CardDescription>
        </CardHeader>
        <CardContent className="h-96">
          <ResponsiveContainer width="100%" height="100%">
            <LineChart data={data} margin={{ top: 8, right: 16, left: 8, bottom: 0 }}>
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
                contentStyle={{
                  background: 'var(--card)',
                  border: '1px solid var(--border)',
                  borderRadius: 8,
                  color: 'var(--foreground)',
                }}
                formatter={(v, name) => [rub(Number(v)), String(name)]}
              />
              {lines.length > 1 && <Legend formatter={(value) => <span style={{ color: 'var(--foreground)' }}>{String(value)}</span>} />}
              {lines.map((name) => (
                <Line
                  key={name}
                  type="monotone"
                  dataKey={name}
                  stroke={name === 'Всего' ? '#10b981' : SERIES_COLORS[name]}
                  strokeWidth={2}
                  dot={{ r: 3.5, strokeWidth: 2, fill: 'var(--card)' }}
                  activeDot={{ r: 5 }}
                />
              ))}
            </LineChart>
          </ResponsiveContainer>
        </CardContent>
      </Card>
    </div>
  )
}
