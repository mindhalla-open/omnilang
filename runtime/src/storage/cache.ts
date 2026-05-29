/**
 * Content-Addressed Storage (CAS) for incremental builds.
 *
 * Caches generated code by hashing the spec input.
 * If the spec hasn't changed, reuse the cached output instead of calling the LLM.
 */

import * as fs from "fs";
import * as path from "path";
import * as crypto from "crypto";

// Lives under the already-gitignored .omni-cache/ root so generated artifacts
// are never accidentally committed.
const CACHE_DIR = ".omni-cache/cas";

export interface CacheEntry {
  hash: string;
  serviceName: string;
  target: string;
  files: Array<{ path: string; content: string }>;
  timestamp: number;
  model: string;
  tokensUsed: number;
}

export class ContentAddressedCache {
  private cacheDir: string;

  constructor(baseDir: string = ".") {
    this.cacheDir = path.join(baseDir, CACHE_DIR);
    fs.mkdirSync(this.cacheDir, { recursive: true });
  }

  /** Compute a BLAKE2-like hash of spec content */
  hash(content: string): string {
    return crypto.createHash("sha256").update(content).digest("hex").substring(0, 16);
  }

  /** Build a cache key from service name, target, and spec content */
  cacheKey(serviceName: string, target: string, specContent: string): string {
    const combined = `${serviceName}:${target}:${specContent}`;
    return this.hash(combined);
  }

  /** Check if a cached entry exists */
  has(key: string): boolean {
    const filePath = path.join(this.cacheDir, `${key}.json`);
    return fs.existsSync(filePath);
  }

  /** Get a cached entry */
  get(key: string): CacheEntry | null {
    const filePath = path.join(this.cacheDir, `${key}.json`);
    if (!fs.existsSync(filePath)) return null;

    try {
      const raw = fs.readFileSync(filePath, "utf8");
      return JSON.parse(raw) as CacheEntry;
    } catch {
      return null;
    }
  }

  /** Store an entry in the cache */
  put(key: string, entry: CacheEntry): void {
    const filePath = path.join(this.cacheDir, `${key}.json`);
    fs.writeFileSync(filePath, JSON.stringify(entry, null, 2), "utf8");
  }

  /** Remove a cached entry */
  remove(key: string): void {
    const filePath = path.join(this.cacheDir, `${key}.json`);
    if (fs.existsSync(filePath)) {
      fs.unlinkSync(filePath);
    }
  }

  /** Clear the entire cache */
  clear(): void {
    if (fs.existsSync(this.cacheDir)) {
      const files = fs.readdirSync(this.cacheDir);
      for (const file of files) {
        fs.unlinkSync(path.join(this.cacheDir, file));
      }
    }
  }

  /** Get cache stats */
  stats(): { entries: number; sizeBytes: number } {
    if (!fs.existsSync(this.cacheDir)) {
      return { entries: 0, sizeBytes: 0 };
    }

    const files = fs.readdirSync(this.cacheDir).filter((f) => f.endsWith(".json"));
    let totalSize = 0;
    for (const file of files) {
      const stat = fs.statSync(path.join(this.cacheDir, file));
      totalSize += stat.size;
    }

    return { entries: files.length, sizeBytes: totalSize };
  }

  /** Detect changes between current and cached spec */
  detectChanges(
    services: Array<{ name: string; specContent: string }>,
    target: string
  ): { changed: string[]; unchanged: string[] } {
    const changed: string[] = [];
    const unchanged: string[] = [];

    for (const svc of services) {
      const key = this.cacheKey(svc.name, target, svc.specContent);
      if (this.has(key)) {
        unchanged.push(svc.name);
      } else {
        changed.push(svc.name);
      }
    }

    return { changed, unchanged };
  }
}
