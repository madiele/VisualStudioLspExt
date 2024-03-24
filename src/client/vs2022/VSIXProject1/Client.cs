using Microsoft.VisualStudio.LanguageServer.Client;
using Microsoft.VisualStudio.Threading;
using Microsoft.VisualStudio.Utilities;
using System;
using System.Collections.Generic;
using System.ComponentModel.Composition;
using System.Diagnostics;
using System.IO;
using System.Net.Mime;
using System.Reflection;
using System.Threading;
using System.Threading.Tasks;

namespace VisualStudioLspExt
{
	[Export(typeof(ILanguageClient))]
	[ContentType("CSharp")]
	internal class Client : ILanguageClient
	{
		public async Task OnLoadedAsync() => await StartAsync.InvokeAsync(this, EventArgs.Empty);
		public Task OnServerInitializedAsync() => Task.CompletedTask;
		public async Task<Connection> ActivateAsync(CancellationToken token)
{
            await Task.Yield();

			var process = new Process
			{
				StartInfo = new ProcessStartInfo
				{
					FileName = Path.Combine(Path.GetDirectoryName(Assembly.GetExecutingAssembly().Location), "server.exe"),
					Arguments = string.Empty,
					RedirectStandardInput = true,
					RedirectStandardOutput = true,
					UseShellExecute = false,
					CreateNoWindow = false,
				}
			};

			if (process.Start())
            {
                return new Connection(process.StandardOutput.BaseStream, process.StandardInput.BaseStream);
            }

            return null;
        }

		public Task<InitializationFailureContext> OnServerInitializeFailedAsync(ILanguageClientInitializationInfo initializationState) 
			=> Task.FromResult(new InitializationFailureContext());

		public string Name => "lsp extension";

		public IEnumerable<string> ConfigurationSections => null;

		public object InitializationOptions => null;

		public IEnumerable<string> FilesToWatch => null;

		public bool ShowNotificationOnInitializeFailed => true;

		public event AsyncEventHandler<EventArgs> StartAsync;
		public event AsyncEventHandler<EventArgs> StopAsync;
	}
}
