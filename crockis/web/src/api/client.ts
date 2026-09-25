import { mockApi } from './mock'
import { restApi } from './rest'
import type { CrockisApi } from './types'

const useMock = import.meta.env.VITE_USE_MOCK !== 'false'

export const api: CrockisApi = useMock ? mockApi : restApi
