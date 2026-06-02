import * as fs from "fs";
import * as os from "os";
import * as path from "path";
import { ContentAddressedCache, CacheEntry } from "../src/storage/cache";

describe("ContentAddressedCache", () => {
  let dir: string;
  let cache: ContentAddressedCache;

  beforeEach(() => {
    dir = fs.mkdtempSync(path.join(os.tmpdir(), "omni-cache-"));
    cache = new ContentAddressedCache(dir);
  });

  afterEach(() => {
    fs.rmSync(dir, { recursive: true, force: true });
  });

  function entry(name: string): CacheEntry {
    return {
      hash: "h",
      serviceName: name,
      target: "typescript",
      files: [{ path: "src/x.ts", content: "export const x = 1;" }],
      timestamp: 0,
      model: "test",
      tokensUsed: 0,
    };
  }

  it("computes a deterministic hash for identical content", () => {
    expect(cache.hash("abc")).toBe(cache.hash("abc"));
    expect(cache.hash("abc")).not.toBe(cache.hash("abd"));
  });

  it("derives the same cache key from the same inputs", () => {
    const k1 = cache.cacheKey("Svc", "typescript", "spec-content");
    const k2 = cache.cacheKey("Svc", "typescript", "spec-content");
    const k3 = cache.cacheKey("Svc", "typescript", "different");
    expect(k1).toBe(k2);
    expect(k1).not.toBe(k3);
  });

  it("round-trips put/get and reports has()", () => {
    const key = cache.cacheKey("Svc", "typescript", "spec");
    expect(cache.has(key)).toBe(false);
    cache.put(key, entry("Svc"));
    expect(cache.has(key)).toBe(true);
    expect(cache.get(key)?.serviceName).toBe("Svc");
    expect(cache.get(key)?.files[0].content).toBe("export const x = 1;");
  });

  it("detects changed vs unchanged services", () => {
    const svc = { name: "A", specContent: "v1" };
    cache.put(cache.cacheKey(svc.name, "typescript", svc.specContent), entry("A"));
    const { changed, unchanged } = cache.detectChanges(
      [svc, { name: "B", specContent: "v1" }],
      "typescript",
    );
    expect(unchanged).toContain("A");
    expect(changed).toContain("B");
  });
});
