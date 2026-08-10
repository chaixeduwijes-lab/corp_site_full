import { useState } from 'react'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../components/ui/card'
import { Badge } from '../components/ui/badge'
import { CALC_MODELS } from '../data/report'

const usd = (v: number) =>
  v.toLocaleString('ru-RU', { maximumFractionDigits: v < 10 ? 2 : 0 })

export default function CostCalculator() {
  const [modelName, setModelName] = useState(CALC_MODELS[0].name)
  const [inTokens, setInTokens] = useState(50) // млн токенов в месяц
  const [outTokens, setOutTokens] = useState(10)

  const model = CALC_MODELS.find((m) => m.name === modelName) ?? CALC_MODELS[0]
  const inCost = inTokens * model.inPrice
  const outCost = outTokens * model.outPrice
  const totalCost = inCost + outCost

  const cheapest = CALC_MODELS.reduce((a, b) =>
    inTokens * a.inPrice + outTokens * a.outPrice <= inTokens * b.inPrice + outTokens * b.outPrice ? a : b,
  )
  const cheapestCost = inTokens * cheapest.inPrice + outTokens * cheapest.outPrice

  return (
    <div className="mx-auto flex max-w-3xl flex-col gap-6">
      <div>
        <h1 className="text-3xl font-bold">Калькулятор затрат</h1>
        <p className="mt-1 text-muted-foreground">
          Месячный бюджет на инференс по ценам из отчёта, $ за 1 млн токенов
        </p>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Модель</CardTitle>
        </CardHeader>
        <CardContent className="flex flex-wrap gap-2">
          {CALC_MODELS.map((m) => (
            <button
              key={m.name}
              onClick={() => setModelName(m.name)}
              aria-pressed={modelName === m.name}
              className={`rounded-md border px-4 py-2.5 text-left text-sm font-medium transition-colors ${
                modelName === m.name
                  ? 'border-secondary bg-secondary/15 text-foreground'
                  : 'border-border bg-card text-muted-foreground hover:text-foreground'
              }`}
            >
              {m.name}
              <span className="ml-2 text-muted-foreground">
                ${m.inPrice} / ${m.outPrice}
              </span>
            </button>
          ))}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Нагрузка в месяц</CardTitle>
          <CardDescription>миллионы токенов</CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col gap-5">
          <label className="flex items-center gap-4">
            <span className="w-40 shrink-0 text-sm text-muted-foreground">
              Входные: <b className="text-foreground">{inTokens} млн</b>
            </span>
            <input
              type="range"
              min={1}
              max={500}
              value={inTokens}
              onChange={(e) => setInTokens(Number(e.target.value))}
              className="w-full accent-[#10b981]"
              aria-label="Входные токены, млн в месяц"
            />
          </label>
          <label className="flex items-center gap-4">
            <span className="w-40 shrink-0 text-sm text-muted-foreground">
              Выходные: <b className="text-foreground">{outTokens} млн</b>
            </span>
            <input
              type="range"
              min={1}
              max={200}
              value={outTokens}
              onChange={(e) => setOutTokens(Number(e.target.value))}
              className="w-full accent-[#10b981]"
              aria-label="Выходные токены, млн в месяц"
            />
          </label>
        </CardContent>
      </Card>

      <Card className="border-secondary">
        <CardHeader>
          <CardTitle>Итого в месяц</CardTitle>
          {model.estimated && (
            <CardDescription>
              <Badge variant="accent">оценка</Badge> цены DeepSeek рассчитаны от «≈99 % дешевле
              флагманов»
            </CardDescription>
          )}
        </CardHeader>
        <CardContent>
          <p className="text-4xl font-bold">${usd(totalCost)}</p>
          <p className="mt-2 text-sm text-muted-foreground">
            вход ${usd(inCost)} + выход ${usd(outCost)} · {model.name}
          </p>
          {cheapest.name !== model.name && (
            <div className="mt-4 border-t border-border pt-4 text-sm">
              <span className="text-muted-foreground">
                Дешевле всего под эту нагрузку — <b className="text-foreground">{cheapest.name}</b>:{' '}
                <b className="text-secondary">${usd(cheapestCost)}</b> (экономия $
                {usd(totalCost - cheapestCost)} в месяц)
              </span>
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  )
}
