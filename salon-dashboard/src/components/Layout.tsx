import type { ReactNode } from 'react'
import { Link, useLocation } from 'wouter'
import { BarChart3, Calculator, LineChart, Newspaper, Users } from 'lucide-react'
import { exportedAt, fmtDate } from '../lib/data'

const NAV = [
  { href: '/', label: 'Дашборд', icon: BarChart3 },
  { href: '/news', label: 'Новости', icon: Newspaper },
  { href: '/masters', label: 'Мастера', icon: Users },
  { href: '/history', label: 'История', icon: LineChart },
  { href: '/calculator', label: 'Калькулятор', icon: Calculator },
]

export default function Layout({ children }: { children: ReactNode }) {
  const [location] = useLocation()

  return (
    <div className="min-h-screen bg-background">
      <header className="sticky top-0 z-40 border-b border-border bg-card/95 backdrop-blur">
        <div className="container flex flex-wrap items-center justify-between gap-x-6 gap-y-2 py-3">
          <Link href="/" className="text-lg font-bold tracking-tight text-foreground">
            Салон<span className="text-secondary">·</span>панель
          </Link>
          <nav aria-label="Основная навигация" className="flex flex-wrap gap-1">
            {NAV.map(({ href, label, icon: Icon }) => {
              const active = href === '/' ? location === '/' : location.startsWith(href)
              return (
                <Link
                  key={href}
                  href={href}
                  className={`inline-flex items-center gap-1.5 rounded-md px-3 py-1.5 text-sm font-medium transition-colors ${
                    active
                      ? 'bg-primary text-primary-foreground'
                      : 'text-muted-foreground hover:bg-muted hover:text-foreground'
                  }`}
                >
                  <Icon size={15} aria-hidden />
                  {label}
                </Link>
              )
            })}
          </nav>
        </div>
      </header>

      <main className="container py-8">{children}</main>

      <footer className="border-t border-border py-6">
        <div className="container text-sm text-muted-foreground">
          Данные: выгрузка salonbackup от {fmtDate(exportedAt.slice(0, 10))} · vest-smr.ru
        </div>
      </footer>
    </div>
  )
}
