import { useState } from 'react'
import { Check, Share2 } from 'lucide-react'
import { Button } from './ui/button'

export function ShareMenu({ title, path }: { title: string; path: string }) {
  const [copied, setCopied] = useState(false)

  async function share() {
    const url = `${location.origin}${path}`
    if (navigator.share) {
      try {
        await navigator.share({ title, url })
        return
      } catch {
        /* пользователь закрыл системное меню — падаем в копирование */
      }
    }
    try {
      await navigator.clipboard.writeText(url)
      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    } catch {
      prompt('Скопируйте ссылку:', url)
    }
  }

  return (
    <Button variant="outline" onClick={share} aria-label={`Поделиться: ${title}`}>
      {copied ? <Check size={16} className="text-secondary" /> : <Share2 size={16} />}
      {copied ? 'Ссылка скопирована' : 'Поделиться'}
    </Button>
  )
}
