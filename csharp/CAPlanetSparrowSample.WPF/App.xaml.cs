using ActiproSoftware.Products;
using Microsoft.Extensions.DependencyInjection;
using System.Configuration;
using System.Data;
using System.Windows;

namespace CAPlanetSparrowSample.WPF
{
    /// <summary>
    /// Interaction logic for App.xaml
    /// </summary>
    public partial class App : Application
    {
        public static IServiceProvider Services { get; private set; } = null!;

        protected override void OnStartup(StartupEventArgs e)
        {
            base.OnStartup(e);

            var services = new ServiceCollection();
            services.AddSingleton<MainWindowViewModel>();
            services.AddSingleton<MainWindow>();
            Services = services.BuildServiceProvider();

            // v2025.1
            ActiproLicenseManager.RegisterLicense(licensee: "CA Planet", licenseKey: "WPF251-9J5YJ-K6YJG-HG4WW-R8GG");

            var _mainWnd = Services.GetRequiredService<MainWindow>();
            this.MainWindow = _mainWnd;
            _mainWnd.Show();
        }
    }

}
