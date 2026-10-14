using System.Text;
using Microsoft.Web.WebView2.Core;
using Microsoft.Web.WebView2.WinForms;

namespace Folio;

static class Program
{
    [STAThread]
    static void Main()
    {
        // One window at a time: a second launch just exits (the first keeps its state).
        using var mutex = new Mutex(true, "Folio.Desktop.SingleInstance", out bool first);
        if (!first) return;
        ApplicationConfiguration.Initialize();
        Application.Run(new MainForm());
    }
}

sealed class MainForm : Form
{
    const string Host = "folio.app";
    readonly WebView2 web = new() { Dock = DockStyle.Fill, DefaultBackgroundColor = Color.FromArgb(244, 244, 247) };
    readonly Hub hub;

    public MainForm()
    {
        Text = "Folio";
        Icon = Icon.ExtractAssociatedIcon(Application.ExecutablePath);
        MinimumSize = new Size(760, 520);
        Size = new Size(1280, 820);
        StartPosition = FormStartPosition.CenterScreen;
        hub = new Hub(Path.Combine(AppContext.BaseDirectory, "catalog.json"));
        Controls.Add(web);
        Load += async (_, _) => await Boot();
    }

    async Task Boot()
    {
        try
        {
            var data = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "Folio", "webview");
            var env = await CoreWebView2Environment.CreateAsync(null, data);
            await web.EnsureCoreWebView2Async(env);
            var core = web.CoreWebView2;
            core.Settings.AreDevToolsEnabled = false;
            core.Settings.IsStatusBarEnabled = false;
            core.Settings.AreDefaultContextMenusEnabled = true;
            // Every request to https://folio.app/ is answered here: /api/* by the native hub,
            // everything else from the bundled web folder (nothing touches the network).
            core.AddWebResourceRequestedFilter($"https://{Host}/*", CoreWebView2WebResourceContext.All);
            core.WebResourceRequested += async (_, e) =>
            {
                var deferral = e.GetDeferral();
                try
                {
                    var path = Uri.UnescapeDataString(new Uri(e.Request.Uri).AbsolutePath);
                    if (path.StartsWith("/api/"))
                    {
                        var (code, body) = await hub.Handle(e.Request.Method, path);
                        e.Response = env.CreateWebResourceResponse(new MemoryStream(Encoding.UTF8.GetBytes(body)), code, "OK",
                            "Content-Type: application/json\r\nCache-Control: no-store");
                    }
                    else e.Response = Serve(env, path);
                }
                finally { deferral.Complete(); }
            };
            // Anything outside the app (release pages, links in documents) opens in the user's browser.
            core.NewWindowRequested += (_, e) => { e.Handled = true; OpenExternal(e.Uri); };
            core.NavigationStarting += (_, e) =>
            {
                if (!Uri.TryCreate(e.Uri, UriKind.Absolute, out var u) || u.Host == Host || u.Scheme is "about" or "data" or "blob") return;
                e.Cancel = true; OpenExternal(e.Uri);
            };
            core.DocumentTitleChanged += (_, _) => Text = string.IsNullOrWhiteSpace(core.DocumentTitle) ? "Folio" : core.DocumentTitle;
            core.Navigate($"https://{Host}/index.html");
        }
        catch (Exception e)
        {
            MessageBox.Show("Folio needs the Microsoft Edge WebView2 Runtime.\n\n" + e.Message, "Folio", MessageBoxButtons.OK, MessageBoxIcon.Warning);
            Close();
        }
    }

    static readonly Dictionary<string, string> Mime = new()
    {
        [".html"] = "text/html; charset=utf-8", [".js"] = "text/javascript; charset=utf-8", [".css"] = "text/css; charset=utf-8",
        [".json"] = "application/json", [".svg"] = "image/svg+xml", [".png"] = "image/png", [".ico"] = "image/x-icon",
    };

    static CoreWebView2WebResourceResponse Serve(CoreWebView2Environment env, string path)
    {
        var root = Path.GetFullPath(Path.Combine(AppContext.BaseDirectory, "web"));
        if (path == "/" || path.EndsWith('/')) path += "index.html";
        var file = Path.GetFullPath(Path.Combine(root, path.TrimStart('/').Replace('/', Path.DirectorySeparatorChar)));
        if (!file.StartsWith(root + Path.DirectorySeparatorChar) || !Mime.TryGetValue(Path.GetExtension(file), out var type) || !File.Exists(file))
            return env.CreateWebResourceResponse(new MemoryStream(Encoding.UTF8.GetBytes("Not found")), 404, "Not Found", "Content-Type: text/plain");
        return env.CreateWebResourceResponse(new MemoryStream(File.ReadAllBytes(file)), 200, "OK", $"Content-Type: {type}\r\nCache-Control: no-store");
    }

    static void OpenExternal(string url)
    {
        if (Uri.TryCreate(url, UriKind.Absolute, out var u) && u.Scheme is "https" or "http" or "mailto")
            System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo(u.ToString()) { UseShellExecute = true });
    }
}
