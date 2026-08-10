import { TrendingDown, TrendingUp } from 'lucide-react'
import { Card, CardContent, CardHeader, CardTitle } from './ui/card'

export function StatCard({
  label,
  value,
  note,
  trend,
}: {
  label: string
  value: string
  note?: string
  trend?: { dir: 'up' | 'down'; text: string; good: boolean }
}) {
  return (
    <Card>
      <CardHeader className="pb-0">
        <CardTitle className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
          {label}
        </CardTitle>
      </CardHeader>
      <CardContent>
        <p className="text-3xl font-bold">{value}</p>
        {trend && (
          <p
            className={`mt-1 inline-flex items-center gap-1 text-sm font-medium ${
              trend.good ? 'text-secondary' : 'text-destructive'
            }`}
          >
            {trend.dir === 'up' ? <TrendingUp size={15} /> : <TrendingDown size={15} />}
            {trend.text}
          </p>
        )}
        {note && <p className="mt-1 text-sm text-muted-foreground">{note}</p>}
      </CardContent>
    </Card>
  )
}
