import { createMemoryBackend } from "./memory-backend"
import { projectStoreContract } from "./project-store.contract"

projectStoreContract("memory", () => Promise.resolve(createMemoryBackend()))
