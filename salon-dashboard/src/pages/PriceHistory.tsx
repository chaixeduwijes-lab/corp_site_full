import {
  CartesianGrid, Legend, Line, LineChart, ResponsiveContainer, Tooltip, XAxis, YAxis,
} from 'recharts'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../components/ui/card'
import { Badge } from '../components/ui/badge'
import { CALC_MODELS, PRICE_HISTORY } from '../data/report'

export default function PriceHistory() {
  return (
    <div className="flex flex-col gap-6">
      <div>
        <h1 className="text-3xl font-bold">История цен</h1>
        <p className="mt-1 text-muted-foreground">
          Стоимость инференса, $ за 1 млн токенов (blended)
        </p>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>DeepSeek против GPT-5.6</CardTitle>
          <CardDescription>июль 2026, $ за 1M токенов</CardDescription>
        </CardHeader>
        <CardContent className="h-96">
          <ResponsiveContainer width="100%" height="100%">
            <LineChart data={PRICE_HISTORY} margin={{ top: 8, right: 16, left: 8, bottom: 0 }}>
              <CartesianGrid strokeDasharray="3 3" stroke="var(--chart-grid)" />
              <XAxis dataKey="date" stroke="var(--chart-axis)" tickLine={false} axisLine={false} />
              <YAxis
                stroke="var(--chart-axis)"
                tickLine={false}
                axisLine={false}
                tickFormatter={(v: number) => `$${v}`}
                width={56}
              />
              <Tooltip
                contentStyle={{
                  background: 'var(--card)',
                  border: '1px solid var(--border)',
                  borderRadius: 8,
                  color: 'var(--foreground)',
                }}
                formatter={(v, name) => [`$${Number(v)} / 1M токенов`, String(name)]}
              />
              <Legend formatter={(value) => <span style={{ color: 'var(--foreground)' }}>{String(value)}</span>} />
              <Line
                type="monotone"
                dataKey="DeepSeek"
                stroke="#10b981"
                strokeWidth={2}
                dot={{ r: 4, strokeWidth: 2, fill: 'var(--card)' }}
              />
              <Line
                type="monotone"
                dataKey="GPT-5.6"
                stroke="#3b82f6"
                strokeWidth={2}
                dot={{ r: 4, strokeWidth: 2, fill: 'var(--card)' }}
              />
            </LineChart>
          </ResponsiveContainer>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Актуальные цены из отчёта</CardTitle>
          <CardDescription>$ за 1 млн токенов: вход / выход</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-border text-left text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                  <th className="py-2 pr-4">Модель</th>
                  <th className="py-2 pr-4 text-right">Вход</th>
                  <th className="py-2 pr-4 text-right">Выход</th>
                  <th className="py-2">Примечание</th>
                </tr>
              </thead>
              <tbody className="[font-variant-numeric:tabular-nums]">
                {CALC_MODELS.map((m) => (
                  <tr key={m.name} className="border-b border-border last:border-0">
                    <td className="py-2.5 pr-4 font-medium">{m.name}</td>
                    <td className="py-2.5 pr-4 text-right">${m.inPrice}</td>
                    <td className="py-2.5 pr-4 text-right">${m.outPrice}</td>
                    <td className="py-2.5 text-muted-foreground">
                      {m.estimated && <Badge variant="accent" className="mr-2">оценка</Badge>}
                      {m.note ?? '—'}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </CardContent>
      </Card>
    </div>
  )
}
