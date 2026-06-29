using ActiproSoftware.Windows;

namespace CAPlanetSparrowSample.WPF.DockSites;

/// <summary>
/// How to use docking mvvm 
/// 1. Read actipro documentation docking mvvm
/// 2. define DocumentViewModel.DockItem
/// 3. define ToolViewModel.DockItem
/// 4. define MainViewModel.DockSite
/// 5. add datatemplate for viewmodel in app.xaml resources
/// 6. add content template for document & tool window in app.xaml resources
/// 7. add docking container style in app.xaml resources
/// </summary>
public class DockSiteViewModel //: ReactiveObject
{
    public DeferrableObservableCollection<DockDocumentItemViewModel> DocumentItems { get; }
    public DeferrableObservableCollection<DockToolItemViewModel> ToolItems { get; }

    public DockSiteViewModel()
    {
        DocumentItems = [];
        ToolItems = [];
    }
}
