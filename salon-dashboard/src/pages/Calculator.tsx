import { useState } from 'react'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../components/ui/card'
import { Badge } from '../components/ui/badge'
import {
  EXTRAS_PRICES, PROGRAMS, PROGRAM_PRICES, entries, num, rub, total,
} from '../lib/data'

export default function Calculator() {
  const [program, setProgram] = useState<string>('Классика')
  const [extras, setExtras] = useState<string[]>([])
  const [visits, setVisits] = useState(1)

  const toggleExtra = (name: string) =>
    setExtras((prev) => (prev.includes(name) ? prev.filter((x) => x !== name) : [...prev, name]))

  const extrasSum = extras.reduce((a, x) => a + EXTRAS_PRICES[x], 0)
  const perVisit = PROGRAM_PRICES[program] + extrasSum
  const totalSum = perVisit * visits

  const programStats = entries.filter((e) => e.program === program)
  const avgCheck = programStats.length
    ? programStats.reduce((a, e) => a + total(e), 0) / programStats.length
    : 0

  return (
    <div className="mx-auto flex max-w-3xl flex-col gap-6">
      <div>
        <h1 className="text-3xl font-bold">Калькулятор стоимости</h1>
        <p className="mt-1 text-muted-foreground">
          Считает стоимость визита по прайсу: программа + дополнительные услуги
        </p>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Программа</CardTitle>
        </CardHeader>
        <CardContent className="flex flex-wrap gap-2">
          {PROGRAMS.map((p) => (
            <button
              key={p}
              onClick={() => setProgram(p)}
              aria-pressed={program === p}
              className={`rounded-md border px-4 py-2.5 text-sm font-medium transition-colors ${
                program === p
                  ? 'border-secondary bg-secondary/15 text-foreground'
                  : 'border-border bg-card text-muted-foreground hover:text-foreground'
              }`}
            >
              {p}
              <span className="ml-2 text-muted-foreground">{rub(PROGRAM_PRICES[p])}</span>
            </button>
          ))}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Дополнительные услуги</CardTitle>
        </CardHeader>
        <CardContent className="flex flex-wrap gap-2">
          {Object.entries(EXTRAS_PRICES).map(([name, price]) => (
            <button
              key={name}
              onClick={() => toggleExtra(name)}
              aria-pressed={extras.includes(name)}
              className={`rounded-md border px-4 py-2.5 text-sm font-medium transition-colors ${
                extras.includes(name)
                  ? 'border-accent bg-accent/15 text-foreground'
                  : 'border-border bg-card text-muted-foreground hover:text-foreground'
              }`}
            >
              {name}
              <span className="ml-2 text-muted-foreground">+{rub(price)}</span>
            </button>
          ))}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Количество визитов</CardTitle>
          <CardDescription>например, абонемент на месяц</CardDescription>
        </CardHeader>
        <CardContent className="flex items-center gap-4">
          <input
            type="range"
            min={1}
            max={12}
            value={visits}
            onChange={(e) => setVisits(Number(e.target.value))}
            className="w-full accent-[#10b981]"
            aria-label="Количество визитов"
          />
          <span className="w-8 text-center text-lg font-bold">{visits}</span>
        </CardContent>
      </Card>

      <Card className="border-secondary">
        <CardHeader>
          <CardTitle>Итого</CardTitle>
        </CardHeader>
        <CardContent>
          <p className="text-4xl font-bold">{rub(totalSum)}</p>
          <p className="mt-2 text-sm text-muted-foreground">
            {rub(perVisit)} за визит × {visits}
            {extras.length > 0 && <> · допуслуги: {extras.join(', ')}</>}
          </p>
          <div className="mt-4 flex flex-wrap items-center gap-2 border-t border-border pt-4 text-sm">
            <Badge variant="secondary">факт</Badge>
            <span className="text-muted-foreground">
              средний чек по «{program}» за 2026 год — {rub(avgCheck)}
              {' '}({num(programStats.length)} визитов)
            </span>
          </div>
        </CardContent>
      </Card>
    </div>
  )
}
