import { Route, Switch } from 'wouter'
import Layout from './components/Layout'
import AiDashboard from './pages/AiDashboard'
import News from './pages/News'
import NewsDetail from './pages/NewsDetail'
import ModelsComparison from './pages/ModelsComparison'
import PriceHistory from './pages/PriceHistory'
import CostCalculator from './pages/CostCalculator'
import SalonDashboard from './pages/SalonDashboard'
import Masters from './pages/Masters'
import History from './pages/History'
import SalonCalculator from './pages/SalonCalculator'

export default function App() {
  return (
    <Layout>
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
    </Layout>
  )
}
