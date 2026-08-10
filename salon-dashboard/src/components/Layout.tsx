import type { ReactNode } from 'react'
import { Link, useLocation } from 'wouter'
import {
  BarChart3, Calculator, Cpu, LineChart, Newspaper, Scissors, Users,
} from 'lucide-react'
import { REPORT } from '../data/report'
import { fmtDate } from '../lib/data'

const MAIN_NAV = [
  { href: '/', label: 'Дашборд', icon: BarChart3 },
  { href: '/news', label: 'Новости', icon: Newspaper },
  { href: '/models', label: 'Модели', icon: Cpu },
  { href: '/prices', label: 'Цены', icon: LineChart },
  { href: '/calculator', label: 'Калькулятор', icon: Calculator },
  { href: '/salon', label: 'Салон', icon: Scissors },
]

const SALON_NAV = [
  { href: '/salon', label: 'Дашборд', icon: BarChart3 },
  { href: '/salon/masters', label: 'Мастера', icon: Users },
  { href: '/salon/history', label: 'История', icon: LineChart },
  { href: '/salon/calculator', label: 'Калькулятор', icon: Calculator },
]

export default function Layout({ children }: { children: ReactNode }) {
  const [location] = useLocation()
  const inSalon = location.startsWith('/salon')

  const isActive = (href: string) => {
    if (href === '/') return location === '/'
    if (href === '/salon') return inSalon
    return location.startsWith(href)
  }

  return (
    <div className="min-h-screen bg-background">
      <header className="sticky top-0 z-40 border-b border-border bg-card/95 backdrop-blur">
        <div className="container flex flex-wrap items-center justify-between gap-x-6 gap-y-2 py-3">
          <Link href="/" className="text-lg font-bold tracking-tight text-foreground">
            Мониторинг<span className="text-secondary">·</span>ИИ
          </Link>
          <nav aria-label="Основная навигация" className="flex flex-wrap gap-1">
            {MAIN_NAV.map(({ href, label, icon: Icon }) => (
              <Link
                key={href}
                href={href}
                className={`inline-flex items-center gap-1.5 rounded-md px-3 py-1.5 text-sm font-medium transition-colors ${
                  isActive(href)
                    ? 'bg-primary text-primary-foreground'
                    : 'text-muted-foreground hover:bg-muted hover:text-foreground'
                }`}
              >
                <Icon size={15} aria-hidden />
                {label}
              </Link>
            ))}
          </nav>
        </div>
        {inSalon && (
          <div className="border-t border-border bg-background/60">
            <nav
              aria-label="Раздел салона"
              className="container flex flex-wrap gap-1 py-1.5"
            >
              {SALON_NAV.map(({ href, label, icon: Icon }) => {
                const active =
                  href === '/salon' ? location === '/salon' : location.startsWith(href)
                return (
                  <Link
                    key={href}
                    href={href}
                    className={`inline-flex items-center gap-1.5 rounded-md px-2.5 py-1 text-xs font-medium transition-colors ${
                      active
                        ? 'bg-secondary/15 text-foreground'
                        : 'text-muted-foreground hover:bg-muted hover:text-foreground'
                    }`}
                  >
                    <Icon size={13} aria-hidden />
                    {label}
                  </Link>
                )
              })}
            </nav>
          </div>
        )}
      </header>

      <main className="container py-8">{children}</main>

      <footer className="border-t border-border py-6">
        <div className="container text-sm text-muted-foreground">
          {inSalon
            ? 'Данные: выгрузка salonbackup от 6 авг 2026 · vest-smr.ru'
            : `${REPORT.author} · выпуск ${fmtDate(REPORT.date)} · vest-smr.ru`}
        </div>
      </footer>
    </div>
  )
}
