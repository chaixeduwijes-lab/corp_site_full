import { Route, Switch } from 'wouter'
import Layout from './components/Layout'
import Dashboard from './pages/Dashboard'
import News from './pages/News'
import NewsDetail from './pages/NewsDetail'
import Masters from './pages/Masters'
import History from './pages/History'
import Calculator from './pages/Calculator'

export default function App() {
  return (
    <Layout>
      <Switch>
        <Route path="/" component={Dashboard} />
        <Route path="/news" component={News} />
        <Route path="/news/:id" component={NewsDetail} />
        <Route path="/masters" component={Masters} />
        <Route path="/history" component={History} />
        <Route path="/calculator" component={Calculator} />
        <Route>
          <div className="container py-16 text-center text-muted-foreground">
            Страница не найдена
          </div>
        </Route>
      </Switch>
    </Layout>
  )
}
