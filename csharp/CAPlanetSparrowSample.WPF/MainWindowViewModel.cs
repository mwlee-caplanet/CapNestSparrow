using ActiproSoftware.Windows.Controls.Docking;
using CAPlanetSparrowSample.WPF.DockSites;
using CAPlanetSparrowSample.WPF.Services;
using CAPlanetSparrowSample.WPF.Views;
using ReactiveUI;
using ReactiveUI.SourceGenerators;
using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.IO;
using System.Reactive.Concurrency;
using System.Text;
using System.Windows.Input;

namespace CAPlanetSparrowSample.WPF
{
    public partial class MainWindowViewModel : ReactiveObject, IDisposable
    {
        public DockSiteViewModel DockSite { get; }

        private ChildObservableCollection<DockDocumentItemViewModel> DockDocuments = [];

        private Dictionary<DockDocumentItemViewModel, WorkspaceViewModel> Workspaces = [];

        [Reactive] public partial WorkspaceViewModel? CurrentWork { get; set; }

        public ICommand ProjectOpenCommand { get; }

        public MainWindowViewModel()
        {
            ProjectOpenCommand = ReactiveCommand.Create(this.ProjectOpen, outputScheduler: RxApp.MainThreadScheduler);
            DockSite = new();
        }

        public void ProjectOpen()
        {
            string _projectPath = CommonDialogService.OpenSparrowInputFileDialog() ?? string.Empty;

            if (_projectPath.Length == 0)
            {
                //실패 메세지
                return;
            }

            var _result = SparrowIOService.InputJsonLoad(Path.GetFileNameWithoutExtension(_projectPath), _projectPath);

            if (_result.IsSuccess == false)
            {
                //실패 메세지
                return;
            }

            var _work = new WorkspaceViewModel(this, _result.InputData!);
            _work.DockingItem.IsOpen = true;
            DockSite.DocumentItems.Add(_work.DockingItem);
            DockDocuments.Add(_work.DockingItem);
            Workspaces[_work.DockingItem] = _work;

            //Project!.Markers.Add(mkMarker);

        }

        public void Dispose()
        {
            
        }
    }
}
