using System.Diagnostics;
using System.Net;
using System.Runtime.InteropServices;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using System.Text.Json.Nodes;
using System.Text.RegularExpressions;

namespace Folio;

/// <summary>Native Folio Hub backend: release lookup, build picking, verified downloads.
/// The hub page talks to it through the same /api/* routes the Node dev server offers.</summary>
public sealed class Hub
{
    const long MaxBytes = 1_500_000_000;
    static readonly string[] CdnHosts = { "objects.githubusercontent.com", "release-assets.githubusercontent.com" };

    readonly HttpClient http = new(new HttpClientHandler { AllowAutoRedirect = true });
    readonly string org;
    readonly JsonArray apps;
    readonly string dataDir = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "Folio");
    readonly Dictionary<string, (string? Etag, JsonObject Data)> cache = new();
    readonly Dictionary<string, JsonObject> jobs = new();

    public Hub(string catalogPath)
    {
        var cat = JsonNode.Parse(File.ReadAllText(catalogPath))!.AsObject();
        org = cat["org"]!.GetValue<string>();
        apps = cat["apps"]!.AsArray();
        http.DefaultRequestHeaders.UserAgent.ParseAdd("folio-hub");
    }

    // ---- routing -------------------------------------------------------------------------------
    public async Task<(int Code, string Body)> Handle(string method, string path)
    {
        try
        {
            if (method == "GET" && path == "/api/apps") return (200, (await ListApps()).ToJsonString());
            var m = Regex.Match(path, "^/api/install/([a-z0-9-]+)$");
            if (method == "POST" && m.Success)
            {
                var app = apps.FirstOrDefault(a => a!["id"]!.GetValue<string>() == m.Groups[1].Value)?.AsObject();
                if (app is null) return (404, "{\"error\":\"unknown app\"}");
                if (jobs.TryGetValue(m.Groups[1].Value, out var j) && j["state"]!.GetValue<string>() == "working")
                    return (409, "{\"error\":\"already running\"}");
                _ = Task.Run(() => Install(app));
                return (202, "{\"started\":true}");
            }
            return (404, "{\"error\":\"not found\"}");
        }
        catch (Exception e) { return (500, new JsonObject { ["error"] = e.Message }.ToJsonString()); }
    }

    // ---- state ---------------------------------------------------------------------------------
    string StateFile => Path.Combine(dataDir, "state.json");

    JsonObject LoadInstalled()
    {
        try { return JsonNode.Parse(File.ReadAllText(StateFile))?["installed"]?.AsObject() ?? new JsonObject(); }
        catch { return new JsonObject(); } // missing or corrupt: nothing recorded
    }

    void SaveInstalled(JsonObject installed)
    {
        Directory.CreateDirectory(dataDir);
        var tmp = StateFile + ".tmp";
        File.WriteAllText(tmp, new JsonObject { ["installed"] = installed }.ToJsonString());
        File.Move(tmp, StateFile, true);
    }

    // ---- releases ------------------------------------------------------------------------------
    async Task<JsonObject> Latest(string repo)
    {
        cache.TryGetValue(repo, out var prev);
        using var req = new HttpRequestMessage(HttpMethod.Get, $"https://api.github.com/repos/{repo}/releases/latest");
        req.Headers.Accept.ParseAdd("application/vnd.github+json");
        if (prev.Etag is not null) req.Headers.TryAddWithoutValidation("If-None-Match", prev.Etag);
        try
        {
            using var res = await http.SendAsync(req);
            if (res.StatusCode == HttpStatusCode.NotModified && prev.Data is not null) return prev.Data;
            if (res.StatusCode == HttpStatusCode.NotFound) return new JsonObject { ["none"] = true };
            if (res.StatusCode is HttpStatusCode.Forbidden or HttpStatusCode.TooManyRequests)
                return new JsonObject { ["error"] = "GitHub rate limit reached; try again later." };
            if (!res.IsSuccessStatusCode) return new JsonObject { ["error"] = $"GitHub answered {(int)res.StatusCode}." };
            var text = await res.Content.ReadAsStringAsync();
            if (text.Length > 2_000_000) return new JsonObject { ["error"] = "GitHub sent an unexpectedly large answer." };
            var j = JsonNode.Parse(text)!;
            var assets = new JsonArray();
            foreach (var a in j["assets"]?.AsArray() ?? new JsonArray())
                assets.Add(new JsonObject
                {
                    ["name"] = a?["name"]?.GetValue<string>() ?? "",
                    ["url"] = a?["browser_download_url"]?.GetValue<string>() ?? "",
                });
            var data = new JsonObject
            {
                ["version"] = j["tag_name"]?.GetValue<string>() ?? "",
                ["notes"] = Truncate(j["body"]?.GetValue<string>() ?? "", 4000),
                ["url"] = j["html_url"]?.GetValue<string>() ?? "",
                ["assets"] = assets,
            };
            cache[repo] = (res.Headers.ETag?.Tag, data);
            return data;
        }
        catch (Exception e) { return prev.Data ?? new JsonObject { ["error"] = $"Could not reach GitHub ({e.Message})." }; }
    }

    static string Truncate(string s, int n) => s.Length <= n ? s : s[..n];

    async Task<JsonArray> ListApps()
    {
        var installed = LoadInstalled();
        var tasks = apps.Select(async a =>
        {
            var app = a!.AsObject();
            var id = app["id"]!.GetValue<string>();
            var latest = await Latest($"{org}/{id}");
            var inst = installed[id]?["version"]?.GetValue<string>();
            var asset = latest["assets"] is JsonArray arr ? PickAsset(arr) : null;
            var o = JsonNode.Parse(app.ToJsonString())!.AsObject();
            o["web"] = app["web"] is null ? null : $"/apps/{app["web"]!.GetValue<string>()}/index.html";
            o["installed"] = inst;
            o["latest"] = latest["version"]?.GetValue<string>();
            o["releaseUrl"] = latest["url"]?.GetValue<string>() ?? $"https://github.com/{org}/{id}/releases";
            o["error"] = latest["error"]?.GetValue<string>();
            o["downloadable"] = asset is not null;
            o["status"] = latest["assets"] is not null && asset is null ? "no-build" : Status(inst, latest);
            o["job"] = jobs.TryGetValue(id, out var jb) ? JsonNode.Parse(jb.ToJsonString()) : null;
            return (JsonNode)o;
        });
        return new JsonArray((await Task.WhenAll(tasks)).ToArray());
    }

    static string Status(string? installed, JsonObject latest)
    {
        if (latest["error"] is not null) return "unknown";
        if (latest["none"] is not null) return "unreleased";
        if (installed is null) return "not-installed";
        return CompareVersions(latest["version"]?.GetValue<string>() ?? "", installed) > 0 ? "update" : "current";
    }

    public static int CompareVersions(string a, string b)
    {
        static int[]? P(string s)
        {
            var m = Regex.Match(s.Trim(), @"^v?(\d+)(?:\.(\d+))?(?:\.(\d+))?");
            return m.Success ? new[] { int.Parse(m.Groups[1].Value), m.Groups[2].Success ? int.Parse(m.Groups[2].Value) : 0, m.Groups[3].Success ? int.Parse(m.Groups[3].Value) : 0 } : null;
        }
        var x = P(a); var y = P(b);
        if (x is null || y is null) return x is not null ? 1 : y is not null ? -1 : 0;
        for (int i = 0; i < 3; i++) if (x[i] != y[i]) return x[i] < y[i] ? -1 : 1;
        return 0;
    }

    // ---- choosing a build ----------------------------------------------------------------------
    public static JsonObject? PickAsset(JsonArray assets, Architecture? arch = null)
    {
        arch ??= RuntimeInformation.OSArchitecture;
        var mine = arch == Architecture.Arm64 ? new[] { "arm64", "aarch64" } : new[] { "x64", "x86_64", "amd64" };
        var other = arch == Architecture.Arm64 ? new[] { "x64", "x86_64", "amd64" } : new[] { "arm64", "aarch64" };
        string[] kinds = { ".msi", ".exe" };
        JsonObject? best = null; int bestScore = 0;
        foreach (var a in assets)
        {
            var name = a?["name"]?.GetValue<string>() ?? "";
            var lower = name.ToLowerInvariant();
            int kind = Array.FindIndex(kinds, k => lower.EndsWith(k));
            if (kind < 0 || Regex.IsMatch(lower, @"sha256|\.sig$|\.asc$")) continue;
            var words = Regex.Split(lower, "[^a-z0-9_]+").Where(w => w.Length > 0).ToArray();
            bool isMine = words.Intersect(mine).Any();
            if (words.Intersect(other).Any() && !isMine) continue;
            int score = 10 - kind + (isMine ? 20 : 0) + (words.Contains("windows") || words.Contains("win64") ? 5 : 0);
            if (score > bestScore) { best = a!.AsObject(); bestScore = score; }
        }
        return best;
    }

    public static Dictionary<string, string> ParseSums(string text)
    {
        var d = new Dictionary<string, string>();
        foreach (Match m in Regex.Matches(text, @"^([0-9a-fA-F]{64})\s+\*?(.+?)\s*$", RegexOptions.Multiline))
            d[m.Groups[2].Value.TrimEnd('\r')] = m.Groups[1].Value.ToLowerInvariant();
        return d;
    }

    // ---- download and install ------------------------------------------------------------------
    public void AssertAllowed(string url)
    {
        var u = new Uri(url);
        bool own = u.Host == "github.com" && u.AbsolutePath.StartsWith($"/{org}/", StringComparison.OrdinalIgnoreCase);
        if (u.Scheme != "https" || !(own || CdnHosts.Contains(u.Host))) throw new InvalidOperationException($"Refusing to download from {u.Host}.");
    }

    async Task Install(JsonObject app)
    {
        var id = app["id"]!.GetValue<string>();
        var job = new JsonObject { ["state"] = "working", ["percent"] = 0, ["error"] = null };
        jobs[id] = job;
        try
        {
            var rel = await Latest($"{org}/{id}");
            if (rel["assets"] is not JsonArray assets) throw new Exception(rel["error"]?.GetValue<string>() ?? "No release has been published yet.");
            var asset = PickAsset(assets) ?? throw new Exception("This release has no build for your computer.");
            var name = asset["name"]!.GetValue<string>();
            if (Path.GetFileName(name) != name) throw new Exception("Unsafe file name.");

            string? expected = null;
            var sums = assets.FirstOrDefault(a => Regex.IsMatch(a?["name"]?.GetValue<string>() ?? "", "^sha256sums(\\.txt)?$", RegexOptions.IgnoreCase));
            if (sums is not null)
            {
                AssertAllowed(sums["url"]!.GetValue<string>());
                var text = await http.GetStringAsync(sums["url"]!.GetValue<string>());
                ParseSums(text.Length > 1_000_000 ? text[..1_000_000] : text).TryGetValue(name, out expected);
            }

            var url = asset["url"]!.GetValue<string>();
            AssertAllowed(url);
            var dir = Path.Combine(dataDir, "downloads");
            Directory.CreateDirectory(dir);
            var dest = Path.Combine(dir, name);
            using var res = await http.GetAsync(url, HttpCompletionOption.ResponseHeadersRead);
            res.EnsureSuccessStatusCode();
            AssertAllowed(res.RequestMessage!.RequestUri!.ToString()); // the final hop after redirects
            var total = res.Content.Headers.ContentLength ?? 0;
            if (total > MaxBytes) throw new Exception("File is larger than the allowed limit.");
            try
            {
                using var sha = SHA256.Create();
                await using (var src = await res.Content.ReadAsStreamAsync())
                await using (var dst = File.Create(dest))
                {
                    var buf = new byte[81920]; long got = 0; int n;
                    while ((n = await src.ReadAsync(buf)) > 0)
                    {
                        got += n;
                        if (got > MaxBytes) throw new Exception("File is larger than the allowed limit.");
                        sha.TransformBlock(buf, 0, n, null, 0);
                        await dst.WriteAsync(buf.AsMemory(0, n));
                        if (total > 0) job["percent"] = (int)(got * 100 / total);
                    }
                    sha.TransformFinalBlock(Array.Empty<byte>(), 0, 0);
                }
                var actual = Convert.ToHexString(sha.Hash!).ToLowerInvariant();
                if (expected is not null && actual != expected) throw new Exception("Checksum mismatch: the file was not kept.");
            }
            catch { try { File.Delete(dest); } catch { } throw; }

            // Hand the verified installer to Windows; the user finishes the install there.
            Process.Start(new ProcessStartInfo(dest) { UseShellExecute = true });
            var installed = LoadInstalled();
            installed[id] = new JsonObject { ["version"] = rel["version"]?.GetValue<string>(), ["verified"] = expected is not null, ["at"] = DateTime.UtcNow.ToString("o") };
            SaveInstalled(installed);
            job["state"] = "done"; job["percent"] = 100;
        }
        catch (Exception e) { job["state"] = "failed"; job["error"] = e.Message; }
    }
}
