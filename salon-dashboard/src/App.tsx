import { lazy, Suspense } from 'react'
import { Route, Switch } from 'wouter'
import Layout from './components/Layout'

/* Страницы грузятся по требованию: recharts и выгрузка салона (388 КБ JSON)
   не попадают в стартовый бандл главной страницы. */
const AiDashboard = lazy(() => import('./pages/AiDashboard'))
const News = lazy(() => import('./pages/News'))
const NewsDetail = lazy(() => import('./pages/NewsDetail'))
const ModelsComparison = lazy(() => import('./pages/ModelsComparison'))
const PriceHistory = lazy(() => import('./pages/PriceHistory'))
const CostCalculator = lazy(() => import('./pages/CostCalculator'))
const SalonDashboard = lazy(() => import('./pages/SalonDashboard'))
const Masters = lazy(() => import('./pages/Masters'))
const History = lazy(() => import('./pages/History'))
const SalonCalculator = lazy(() => import('./pages/SalonCalculator'))

function PageFallback() {
  return (
    <div className="py-16 text-center text-sm text-muted-foreground" role="status">
      Загрузка…
    </div>
  )
}

export default function App() {
  return (
    <Layout>
      <Suspense fallback={<PageFallback />}>
        <Switch>
          <Route path="/" component={AiDashboard} />
          <Route path="/news" component={News} />
          <Route path="/news/:id" component={NewsDetail} />
          <Route path="/models" component={ModelsComparison} />
          <Route path="/prices" component={PriceHistory} />
          <Route path="/calculator" component={CostCalculator} />
          <Route path="/salon" component={SalonDashboard} />
          <Route path="/salon/masters" component={Masters} />
          <Route path="/salon/history" component={History} />
          <Route path="/salon/calculator" component={SalonCalculator} />
          <Route>
            <div className="container py-16 text-center text-muted-foreground">
              Страница не найдена
            </div>
          </Route>
        </Switch>
      </Suspense>
    </Layout>
  )
}
