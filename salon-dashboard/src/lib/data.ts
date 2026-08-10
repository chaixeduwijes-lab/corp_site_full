import raw from '../data/salon.json'

export interface Entry {
  id: string
  date: string
  time: string
  admin: string
  master: string
  program: string
  price: number
  extras: number
  extrasNote: string
  pay: 'cash' | 'card' | 'transfer'
  vip: boolean
  newGuest: boolean
  comment: string
}
export interface Expense {
  id: string
  date: string
  category: string
  amount: number
  source: 'cash' | 'account'
  note: string
}
export interface Master {
  id: string
  name: string
  percent: number
  rating: number
}
export interface TaxItem {
  id: string
  title: string
  amount: number
  deadline: string
  desc: string
  paid: boolean
}
export interface NewsItem {
  id: string
  date: string
  tag: string
  title: string
  text: string
}

interface Backup {
  entries: Entry[]
  expenses: Expense[]
  masters: Master[]
  tax_calendar: TaxItem[]
  news: Omit<NewsItem, 'id'>[]
  settings: { monthGoal: number; cashStart: number; accountStart: number }
  exportedAt: string
}

const backup = raw as unknown as Backup

export const entries: Entry[] = [...backup.entries].sort((a, b) =>
  (a.date + a.time).localeCompare(b.date + b.time),
)
export const expenses: Expense[] = backup.expenses
export const masters: Master[] = backup.masters
export const taxCalendar: TaxItem[] = backup.tax_calendar
export const news: NewsItem[] = backup.news.map((n, i) => ({ ...n, id: `n${i + 1}` }))
export const settings = backup.settings
export const exportedAt = backup.exportedAt

export const total = (e: Entry) => e.price + e.extras

export const PROGRAMS = ['Классика', 'Для него', 'VIP', 'Для неё'] as const
export const PROGRAM_PRICES: Record<string, number> = {
  'Классика': 3500,
  'Для него': 4500,
  'VIP': 9000,
  'Для неё': 5500,
}
export const EXTRAS_PRICES: Record<string, number> = {
  'Массаж спины': 1000,
  'Парафинотерапия': 700,
  'Пилинг': 500,
}
export const PAY_LABELS: Record<Entry['pay'], string> = {
  card: 'Карта',
  transfer: 'Перевод',
  cash: 'Наличные',
}

/* Цвета серий — изумруд/янтарь/сланцевый/фиолет, читаются в обеих темах */
export const SERIES_COLORS: Record<string, string> = {
  'Классика': '#3b82f6',
  'Для него': '#10b981',
  'VIP': '#f59e0b',
  'Для неё': '#8b5cf6',
}

export const monthsInData: string[] = [...new Set(entries.map((e) => e.date.slice(0, 7)))].sort()

export const MONTH_FULL = [
  'Январь', 'Февраль', 'Март', 'Апрель', 'Май', 'Июнь',
  'Июль', 'Август', 'Сентябрь', 'Октябрь', 'Ноябрь', 'Декабрь',
]
export const MONTH_SHORT = [
  'янв', 'фев', 'мар', 'апр', 'май', 'июн',
  'июл', 'авг', 'сен', 'окт', 'ноя', 'дек',
]

const nf = new Intl.NumberFormat('ru-RU')
export const rub = (v: number) => `${nf.format(Math.round(v))} ₽`
export const num = (v: number) => nf.format(v)

export function pluralRu(n: number, one: string, few: string, many: string): string {
  const m10 = n % 10
  const m100 = n % 100
  if (m10 === 1 && m100 !== 11) return one
  if (m10 >= 2 && m10 <= 4 && (m100 < 10 || m100 >= 20)) return few
  return many
}

export function fmtDate(iso: string): string {
  const d = new Date(iso.length === 10 ? iso + 'T12:00:00' : iso)
  return `${d.getDate()} ${MONTH_SHORT[d.getMonth()]} ${d.getFullYear()}`
}

export function monthLabel(ym: string): string {
  const [y, m] = ym.split('-')
  return `${MONTH_FULL[Number(m) - 1]} ${y}`
}
