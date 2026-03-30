export interface GameEntry {
  id: string
  title: string
  author: string
  description: string
  thumbnailPath: string
  executablePath: string
  version: string
  enabled: boolean
}

export interface AppConfig {
  adminPin: string
  mamePath: string
  mameArgs: string[]
  gamesDir: string
}
